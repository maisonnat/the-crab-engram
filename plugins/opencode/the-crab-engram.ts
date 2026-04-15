const BASE_URL = "http://localhost:7437";

interface EngramPluginState {
  baseUrl: string;
  sessionId: string | null;
  projectName: string;
  lastInjection: number;
  pendingContext: string | null;
  serverHealthy: boolean;
}

const state: EngramPluginState = {
  baseUrl: BASE_URL,
  sessionId: null,
  projectName: "default",
  lastInjection: 0,
  pendingContext: null,
  serverHealthy: false,
};

async function healthCheck(): Promise<boolean> {
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);
    const resp = await fetch(`${state.baseUrl}/health`, {
      signal: controller.signal,
    });
    clearTimeout(timeout);
    state.serverHealthy = resp.ok;
    return resp.ok;
  } catch {
    state.serverHealthy = false;
    return false;
  }
}

async function ensureServer($: any): Promise<boolean> {
  if (await healthCheck()) return true;

  try {
    $.spawn("the-crab-engram", ["serve", "--port", "7437"], {
      background: true,
    });
  } catch {
    console.warn("[engram] Failed to spawn server");
    return false;
  }

  for (let i = 0; i < 3; i++) {
    await new Promise((r) => setTimeout(r, 2000));
    if (await healthCheck()) return true;
  }

  console.warn("[engram] Server failed to start after 3 retries");
  return false;
}

export default {
  server: async (input: any, options: any) => {
    const project =
      input.project || input.directory?.split("/").pop() || "default";
    state.projectName = project;

    return {
      async event(event: any) {
        if (event.type === "session.created") {
          await ensureServer(input.$);
          try {
            const resp = await fetch(`${state.baseUrl}/sessions`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ project }),
            });
            const data = await resp.json();
            state.sessionId = data.session_id;
          } catch {
            console.warn("[engram] Failed to create session");
          }
        }

        if (event.type === "session.idle") {
          try {
            await fetch(`${state.baseUrl}/consolidate`, { method: "POST" });
          } catch {}
        }

        if (event.type === "session.deleted" && state.sessionId) {
          try {
            await fetch(`${state.baseUrl}/sessions/${state.sessionId}/end`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ summary: "session deleted" }),
            });
          } catch {
          } finally {
            state.sessionId = null;
          }
        }

        if (event.type === "file.edited" && event.file) {
          try {
            await fetch(`${state.baseUrl}/observations`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                title: `File edited: ${event.file}`,
                content: event.file,
                type: "file_change",
                session_id: state.sessionId || "unknown",
                project: state.projectName,
              }),
            });
          } catch {}
        }
      },

      async ["tool.execute.after"](ctx: any) {
        const toolName = ctx?.tool || "";
        if (toolName !== "bash" && toolName !== "shell") return;

        const output = ctx?.output || "";
        if (!output) return;

        const now = Date.now();
        if (state.lastInjection && now - state.lastInjection < 2000) return;

        try {
          if (
            output.includes("git commit") ||
            output.includes("git-commit")
          ) {
            state.lastInjection = now;
            await fetch(`${state.baseUrl}/observations`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                title: "Git commit detected",
                content: output.substring(0, 500),
                type: "file_change",
                session_id: state.sessionId || "unknown",
                project: state.projectName,
              }),
            });
          }

          const isError =
            output.includes("error") || output.includes("failed");
          const hasNonZeroExit =
            ctx?.exitCode !== undefined && ctx.exitCode !== 0;
          if (isError && hasNonZeroExit) {
            state.lastInjection = now;
            await fetch(`${state.baseUrl}/observations`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                title: "Error captured",
                content: output.substring(0, 500),
                type: "bugfix",
                session_id: state.sessionId || "unknown",
                project: state.projectName,
              }),
            });
          }
        } catch {}
      },

      async ["experimental.session.compacting"](ctx: any) {
        if (!state.serverHealthy) return;

        try {
          const [contextResp, capsulesResp, antipatternsResp] =
            await Promise.all([
              fetch(`${state.baseUrl}/context?limit=10`),
              fetch(`${state.baseUrl}/capsules`),
              fetch(`${state.baseUrl}/antipatterns`),
            ]);

          if (contextResp.ok) {
            const data = await contextResp.json();
            const md = formatContextAsMarkdown(data);
            ctx.output.context.push(md);
          }

          if (capsulesResp.ok) {
            const data = await capsulesResp.json();
            if (Array.isArray(data) && data.length > 0) {
              const md = data
                .map((c: any) => `## ${c.topic}\n${c.content || ""}`)
                .join("\n\n");
              ctx.output.context.push(`## Knowledge Capsules\n${md}`);
            }
          }

          if (antipatternsResp.ok) {
            const data = await antipatternsResp.json();
            if (Array.isArray(data) && data.length > 0) {
              const md = data
                .map(
                  (p: any) =>
                    `- **${p.type}** (${p.severity}): ${p.description}`
                )
                .join("\n");
              ctx.output.context.push(`## Anti-Patterns\n${md}`);
            }
          }

          ctx.output.context.push(
            "## Recovery Instructions\n" +
              "After compaction, call `mem_context` first to recover recent session context.\n" +
              "Then call `mem_session_summary` with what was accomplished before compaction."
          );
        } catch {
          console.warn("[engram] Failed to inject compaction context");
        }
      },

      async ["experimental.chat.system.transform"](ctx: any) {
        if (state.pendingContext) {
          ctx.output.system.push(state.pendingContext);
          state.pendingContext = null;
        }
      },

      async ["chat.message"](ctx: any) {
        if (!state.serverHealthy) return;

        const text = extractText(ctx.message);
        if (!text || text.length < 5) return;

        const now = Date.now();
        if (state.lastInjection && now - state.lastInjection < 30000) return;

        try {
          const searchResp = await fetch(`${state.baseUrl}/search`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ query: text, limit: 1 }),
          });

          if (!searchResp.ok) return;
          const results = await searchResp.json();
          if (!Array.isArray(results) || results.length === 0) return;

          const budget = parseInt(
            process.env.ENGRAM_INJECT_BUDGET || "2000",
            10
          );
          const injectResp = await fetch(`${state.baseUrl}/inject`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              task: text,
              max_tokens: budget,
            }),
          });

          if (injectResp.ok) {
            const data = await injectResp.json();
            if (data.markdown) {
              state.pendingContext = data.markdown;
              state.lastInjection = now;
            }
          }
        } catch {}
      },
    };
  },
};

function extractText(message: any): string {
  if (typeof message === "string") return message;
  if (message?.content) {
    if (typeof message.content === "string") return message.content;
    if (Array.isArray(message.content)) {
      return message.content
        .filter((b: any) => b.type === "text")
        .map((b: any) => b.text)
        .join(" ");
    }
  }
  return "";
}

function formatContextAsMarkdown(data: any): string {
  const lines: string[] = ["## Recent Session Context\n"];
  if (data.session) {
    lines.push(
      `Session: ${data.session.id?.substring(0, 8) || "unknown"} (started ${data.session.started_at || "unknown"})\n`
    );
  }
  if (Array.isArray(data.observations) && data.observations.length > 0) {
    for (const obs of data.observations) {
      lines.push(
        `- #${obs.id} [${obs.type}] ${obs.title} — ${(obs.content || "").substring(0, 100)}`
      );
    }
  } else {
    lines.push("No recent observations.");
  }
  return lines.join("\n");
}
