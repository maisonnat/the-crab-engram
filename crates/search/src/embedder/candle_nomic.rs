//! Candle-based embedder using nomic-embed-text-v1.5.
//!
//! Implements a minimal NomicBertModel with RoPE, SwiGLU gated MLP,
//! mean pooling, L2 normalization, and 384d Matryoshka output.
//!
//! Weights: https://huggingface.co/nomic-ai/nomic-embed-text-v1.5
//!
//! Note: This file is only compiled when `feature = "candle"` is enabled
//! (guarded by `#[cfg(feature = "candle")]` in mod.rs).

use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::{Embedding, LayerNorm, Module, VarBuilder, embedding, layer_norm, linear_no_bias};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;
use tracing::info;

use super::{Embedder, EmbedderError};

// ─── Constants from model config.json ──────────────────────────────────
const VOCAB_SIZE: usize = 30528;
const HIDDEN_SIZE: usize = 768;
const NUM_LAYERS: usize = 12;
const NUM_HEADS: usize = 12;
const HEAD_DIM: usize = HIDDEN_SIZE / NUM_HEADS; // 64
const INTERMEDIATE_SIZE: usize = 3072;
const MAX_POSITIONS: usize = 8192;
const PAD_TOKEN: u32 = 0;
const EMBED_DIM: usize = 384;
const ROPE_THETA: f64 = 1000.0;
const LAYER_NORM_EPS: f64 = 1e-12;

/// Convert candle errors to EmbedderError.
fn candle_err(e: impl std::fmt::Display) -> EmbedderError {
    EmbedderError::Generation(format!("Candle error: {e}"))
}

// ═══════════════════════════════════════════════════════════════════════
// RoPE helpers
// ═══════════════════════════════════════════════════════════════════════

/// Build cos/sin cache for rotary embeddings.
fn build_rope(device: &Device) -> candle_core::Result<(Tensor, Tensor)> {
    let inv_freq: Vec<f32> = (0..HEAD_DIM)
        .step_by(2)
        .map(|i| (1.0 / ROPE_THETA.powf(i as f64 / HEAD_DIM as f64)) as f32)
        .collect();
    let inv_freq = Tensor::from_vec(inv_freq, (HEAD_DIM / 2,), device)?;
    let positions = Tensor::arange(0u32, MAX_POSITIONS as u32, device)?.to_dtype(DType::F32)?;
    let positions = positions.reshape((MAX_POSITIONS, 1))?;
    let freqs = positions.matmul(&inv_freq.reshape((1, HEAD_DIM / 2))?)?;
    Ok((freqs.cos()?, freqs.sin()?))
}

/// Apply rotary embeddings to [b, s, nh, hd] tensor.
fn apply_rope(
    xs: &Tensor,
    cos: &Tensor,
    sin: &Tensor,
    seq_len: usize,
) -> candle_core::Result<Tensor> {
    let dims = xs.dims();
    if dims.len() != 4 || seq_len == 0 {
        return Ok(xs.clone());
    }
    let hd = dims[3];
    let half = hd / 2;
    let xs_l = xs.narrow(3, 0, half)?;
    let xs_r = xs.narrow(3, half, half)?;
    let rotated = Tensor::cat(&[xs_r.neg()?, xs_l], 3)?;

    let cos_s = cos.narrow(0, 0, seq_len)?.reshape((1, seq_len, 1, half))?;
    let sin_s = sin.narrow(0, 0, seq_len)?.reshape((1, seq_len, 1, half))?;

    // Duplicate cos/sin along last dim to cover full head_dim
    let cos_e = Tensor::cat(&[cos_s.clone(), cos_s], 3)?;
    let sin_e = Tensor::cat(&[sin_s.clone(), sin_s], 3)?;

    Ok((xs.broadcast_mul(&cos_e)? + rotated.broadcast_mul(&sin_e)?)?)
}

// ═══════════════════════════════════════════════════════════════════════
// Model components
// ═══════════════════════════════════════════════════════════════════════

struct NomicBertEmbeddings {
    word_embeddings: Embedding,
    token_type_embeddings: Embedding,
    ln: LayerNorm,
}

impl NomicBertEmbeddings {
    fn load(vb: VarBuilder) -> candle_core::Result<Self> {
        Ok(Self {
            word_embeddings: embedding(VOCAB_SIZE, HIDDEN_SIZE, vb.pp("word_embeddings"))?,
            token_type_embeddings: embedding(2, HIDDEN_SIZE, vb.pp("token_type_embeddings"))?,
            ln: layer_norm(HIDDEN_SIZE, LAYER_NORM_EPS, vb.pp("emb_ln"))?,
        })
    }

    fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor) -> candle_core::Result<Tensor> {
        let we = self.word_embeddings.forward(input_ids)?;
        let te = self.token_type_embeddings.forward(token_type_ids)?;
        self.ln.forward(&we.add(&te)?)
    }
}

struct NomicBertGatedMLP {
    fc11: candle_nn::Linear,
    fc12: candle_nn::Linear,
    fc2: candle_nn::Linear,
}

impl NomicBertGatedMLP {
    fn load(vb: VarBuilder) -> candle_core::Result<Self> {
        Ok(Self {
            fc11: linear_no_bias(INTERMEDIATE_SIZE, HIDDEN_SIZE, vb.pp("fc11"))?,
            fc12: linear_no_bias(INTERMEDIATE_SIZE, HIDDEN_SIZE, vb.pp("fc12"))?,
            fc2: linear_no_bias(HIDDEN_SIZE, INTERMEDIATE_SIZE, vb.pp("fc2"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        // SwiGLU: silu(fc12(x)) * fc11(x) → fc2
        let y = self.fc11.forward(x)?;
        let gate = self.fc12.forward(x)?.silu()?;
        self.fc2.forward(&y.mul(&gate)?)
    }
}

struct NomicBertAttention {
    wqkv: candle_nn::Linear,
    out_proj: candle_nn::Linear,
    cos: Tensor,
    sin: Tensor,
}

impl NomicBertAttention {
    fn load(vb: VarBuilder, cos: Tensor, sin: Tensor) -> candle_core::Result<Self> {
        Ok(Self {
            wqkv: linear_no_bias(NUM_HEADS * HEAD_DIM * 3, HIDDEN_SIZE, vb.pp("Wqkv"))?,
            out_proj: linear_no_bias(HIDDEN_SIZE, HIDDEN_SIZE, vb.pp("out_proj"))?,
            cos,
            sin,
        })
    }

    fn forward(&self, x: &Tensor, mask: Option<&Tensor>) -> candle_core::Result<Tensor> {
        let (b, s, _) = x.dims3()?;

        // Fused QKV projection
        let qkv = self.wqkv.forward(x)?; // [b, s, 2304]
        let qkv = qkv.reshape((b, s, 3, NUM_HEADS, HEAD_DIM))?;
        let q = qkv.narrow(2, 0, 1)?.squeeze(2)?; // [b, s, nh, hd]
        let k = qkv.narrow(2, 1, 1)?.squeeze(2)?;
        let v = qkv.narrow(2, 2, 1)?.squeeze(2)?;

        // RoPE on Q and K
        let q = apply_rope(&q, &self.cos, &self.sin, s)?;
        let k = apply_rope(&k, &self.cos, &self.sin, s)?;

        // Transpose: [b, nh, s, hd]
        let q = q.permute((0, 2, 1, 3))?;
        let k = k.permute((0, 2, 1, 3))?;
        let v = v.permute((0, 2, 1, 3))?;

        let scale = Tensor::new(1.0 / (HEAD_DIM as f64).sqrt(), x.device())?;
        let attn = q.matmul(&k.transpose(2, 3)?)?;
        let attn = attn.broadcast_mul(&scale)?;

        let attn = if let Some(m) = mask {
            candle_nn::ops::softmax(&(attn.add(&m)?), 3)?
        } else {
            candle_nn::ops::softmax(&attn, 3)?
        };

        let out = attn.matmul(&v)?;
        let out = out.permute((0, 2, 1, 3))?.reshape((b, s, HIDDEN_SIZE))?;
        self.out_proj.forward(&out)
    }
}

struct NomicBertBlock {
    norm1: LayerNorm,
    attn: NomicBertAttention,
    norm2: LayerNorm,
    mlp: NomicBertGatedMLP,
}

impl NomicBertBlock {
    fn load(vb: VarBuilder, cos: Tensor, sin: Tensor) -> candle_core::Result<Self> {
        Ok(Self {
            norm1: layer_norm(HIDDEN_SIZE, LAYER_NORM_EPS, vb.pp("norm1"))?,
            attn: NomicBertAttention::load(vb.pp("attn"), cos, sin)?,
            norm2: layer_norm(HIDDEN_SIZE, LAYER_NORM_EPS, vb.pp("norm2"))?,
            mlp: NomicBertGatedMLP::load(vb.pp("mlp"))?,
        })
    }

    fn forward(&self, x: &Tensor, mask: Option<&Tensor>) -> candle_core::Result<Tensor> {
        // Post-norm: Attn → residual → norm1 → MLP → residual → norm2
        let a = self.attn.forward(x, mask)?;
        let x = self.norm1.forward(&(a + x)?)?;
        let m = self.mlp.forward(&x)?;
        self.norm2.forward(&m.add(&x)?)
    }
}

struct NomicBertEncoder {
    layers: Vec<NomicBertBlock>,
}

impl NomicBertEncoder {
    fn load(vb: VarBuilder, cos: Tensor, sin: Tensor) -> candle_core::Result<Self> {
        let mut layers = Vec::with_capacity(NUM_LAYERS);
        for i in 0..NUM_LAYERS {
            layers.push(NomicBertBlock::load(
                vb.pp(&format!("layers.{i}")),
                cos.clone(),
                sin.clone(),
            )?);
        }
        Ok(Self { layers })
    }

    fn forward(&self, x: &Tensor, mask: Option<&Tensor>) -> candle_core::Result<Tensor> {
        let mut h = x.clone();
        for layer in &self.layers {
            h = layer.forward(&h, mask)?;
        }
        Ok(h)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Public embedder
// ═══════════════════════════════════════════════════════════════════════

/// Candle-powered embedder using nomic-embed-text-v1.5 (384d).
///
/// Downloads the safetensors model (~550 MB) and tokenizer from HuggingFace
/// on first use. Cached at `~/.cache/huggingface/hub/`.
pub struct CandleNomicEmbedder {
    embeddings: NomicBertEmbeddings,
    encoder: NomicBertEncoder,
    tokenizer: Tokenizer,
    device: Device,
}

impl CandleNomicEmbedder {
    /// Download and load the model (CPU only).
    pub fn new() -> Result<Self, EmbedderError> {
        let device = Device::Cpu;
        let api =
            Api::new().map_err(|e| EmbedderError::NotInitialized(format!("HF Hub init: {e}")))?;
        let repo = api.model("nomic-ai/nomic-embed-text-v1.5".to_string());

        let weights = repo
            .get("model.safetensors")
            .map_err(|e| EmbedderError::NotInitialized(format!("download weights: {e}")))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .map_err(|e| EmbedderError::NotInitialized(format!("download tokenizer: {e}")))?;

        info!("Loading nomic-embed-text-v1.5 from {}", weights.display());

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| EmbedderError::NotInitialized(format!("tokenizer: {e}")))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights], DType::F32, &device)
                .map_err(|e| EmbedderError::NotInitialized(format!("safetensors: {e}")))?
        };

        let (cos, sin) =
            build_rope(&device).map_err(|e| EmbedderError::NotInitialized(format!("RoPE: {e}")))?;

        let embeddings = NomicBertEmbeddings::load(vb.pp("embeddings"))
            .map_err(|e| EmbedderError::NotInitialized(format!("embeddings: {e}")))?;
        let encoder = NomicBertEncoder::load(vb.pp("encoder"), cos, sin)
            .map_err(|e| EmbedderError::NotInitialized(format!("encoder: {e}")))?;

        info!("CandleNomicEmbedder ready: nomic-embed-text-v1.5 ({EMBED_DIM}d)");
        Ok(Self {
            embeddings,
            encoder,
            tokenizer,
            device,
        })
    }

    /// Generate a 384-dimensional embedding vector.
    pub fn embed_inner(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| EmbedderError::Generation(format!("tokenize: {e}")))?;

        let ids: Vec<u32> = encoding.get_ids().iter().map(|&i| i as u32).collect();
        let seq_len = ids.len();
        if seq_len == 0 {
            return Ok(vec![0.0; EMBED_DIM]);
        }

        let ids_t = Tensor::from_slice(&ids, (1, seq_len), &self.device).map_err(&candle_err)?;
        let tt_t = Tensor::zeros((1, seq_len), DType::U32, &self.device).map_err(&candle_err)?;

        // Attention mask: 1.0 for non-padding, 0.0 for padding
        let pad_t = Tensor::new(PAD_TOKEN as u32, &self.device).map_err(&candle_err)?;
        let mask_t: Tensor = ids_t
            .broadcast_ne(&pad_t)
            .map_err(&candle_err)?
            .to_dtype(DType::F32)
            .map_err(&candle_err)?;

        let h = self
            .embeddings
            .forward(&ids_t, &tt_t)
            .map_err(&candle_err)?;
        let h = self.encoder.forward(&h, None).map_err(&candle_err)?;

        // Mean pooling over non-padding tokens
        let mask_3d = mask_t.reshape((1, seq_len, 1)).map_err(&candle_err)?;
        let summed = (h * &mask_3d)
            .map_err(&candle_err)?
            .sum(1)
            .map_err(&candle_err)?;
        let count = mask_t
            .sum(1)
            .map_err(&candle_err)?
            .reshape((1, 1))
            .map_err(&candle_err)?;
        let one = Tensor::new(1f32, &self.device).map_err(&candle_err)?;
        let count = count.maximum(&one).map_err(&candle_err)?;
        let pooled = summed.broadcast_div(&count).map_err(&candle_err)?;

        // L2 normalize
        let norm = pooled
            .sqr()
            .map_err(&candle_err)?
            .sum(1)
            .map_err(&candle_err)?
            .sqrt()
            .map_err(&candle_err)?;
        let eps = Tensor::new(1e-12f32, &self.device).map_err(&candle_err)?;
        let pooled = pooled
            .broadcast_div(&norm.maximum(&eps).map_err(&candle_err)?)
            .map_err(&candle_err)?;

        // Matryoshka: slice to EMBED_DIM (384)
        let embedding = pooled.narrow(1, 0, EMBED_DIM).map_err(&candle_err)?;
        embedding
            .to_vec1::<f32>()
            .map_err(|e| EmbedderError::Generation(e.to_string()))
    }
}

#[async_trait]
impl Embedder for CandleNomicEmbedder {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        self.embed_inner(text)
    }

    fn target_dimensions(&self) -> usize {
        EMBED_DIM
    }

    fn model_name(&self) -> &str {
        "nomic-embed-text-v1.5"
    }
}
