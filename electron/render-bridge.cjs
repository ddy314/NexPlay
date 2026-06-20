const fs = require("node:fs");
const path = require("node:path");
const { fork } = require("node:child_process");

class RenderBridge {
  constructor({ projectRoot }) {
    this.projectRoot = projectRoot;
    this.renderDaemon = null;
    this.nextRequestId = 1;
    this.pending = new Map();
    this.info = null;
  }

  async getInfo() {
    if (this.info) {
      return this.info;
    }

    try {
      const info = await this.request({ type: "info" });
      this.info = {
        ...info,
        available: true,
      };
      return this.info;
    } catch (error) {
      this.info = {
        available: false,
        reason: error instanceof Error ? error.message : String(error),
      };
      return this.info;
    }
  }

  async probeWebglTextureRenderer() {
    try {
      return await this.request({ type: "probeWebglTextureRenderer" });
    } catch (error) {
      return {
        ok: false,
        stage: "electronBridge",
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  request(command) {
    const child = this.ensureStarted();
    const id = this.nextRequestId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      child.send({ id, command }, (error) => {
        if (error) {
          this.pending.delete(id);
          reject(error);
        }
      });
    });
  }

  ensureStarted() {
    if (this.renderDaemon && !this.renderDaemon.killed && this.renderDaemon.connected) {
      return this.renderDaemon;
    }

    const daemonPath = path.join(this.projectRoot, "native/mpv-render-bridge/renderer-daemon.cjs");
    const addonPath = path.join(this.projectRoot, "native/mpv-render-bridge/build/Release/mpv_render_bridge.node");
    if (!fs.existsSync(daemonPath) || !fs.existsSync(addonPath)) {
      throw new Error("native render bridge is not built");
    }

    const child = fork(daemonPath, [], {
      cwd: this.projectRoot,
      env: process.env,
      execPath: process.env.NEXPLAY_NODE_BIN || "node",
      serialization: "advanced",
      stdio: ["ignore", "pipe", "pipe", "ipc"],
    });

    this.renderDaemon = child;
    child.stdout.on("data", (chunk) => {
      const text = chunk.toString().trim();
      if (text) {
        console.log(`[mpv-render] ${text}`);
      }
    });
    child.stderr.on("data", (chunk) => {
      const text = chunk.toString().trim();
      if (text) {
        console.warn(`[mpv-render] ${text}`);
      }
    });
    child.on("message", (message) => {
      const pending = this.pending.get(message?.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.ok) {
        pending.resolve(message.payload);
      } else {
        pending.reject(new Error(message.error || "native render bridge request failed"));
      }
    });
    child.on("close", () => {
      if (this.renderDaemon === child) {
        this.renderDaemon = null;
        this.info = null;
      }
      this.rejectPending(new Error("native render bridge exited"));
    });
    child.on("error", (error) => {
      this.rejectPending(error);
    });

    return child;
  }

  rejectPending(error) {
    for (const pending of this.pending.values()) {
      pending.reject(error);
    }
    this.pending.clear();
  }

  shutdown() {
    if (this.renderDaemon) {
      this.renderDaemon.kill();
      this.renderDaemon = null;
    }
    this.rejectPending(new Error("native render bridge stopped"));
  }
}

module.exports = {
  RenderBridge,
};
