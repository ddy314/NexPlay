const { EventEmitter } = require("node:events");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

function packagedBackendPath(projectRoot) {
  const executableName = process.platform === "win32" ? "nexplay.exe" : "nexplay";
  const candidate = path.join(projectRoot, "backend", executableName);
  return fs.existsSync(candidate) ? candidate : null;
}

function backendArgs(command, projectRoot = path.join(__dirname, "..")) {
  const backendBin = process.env.NEXPLAY_BACKEND_BIN;
  const packagedBackend = backendBin || packagedBackendPath(projectRoot);
  const executable = packagedBackend || "cargo";
  const args = packagedBackend ? [command] : ["run", "--quiet", "--", command];
  return { executable, args };
}

class BackendRpcClient extends EventEmitter {
  constructor({ projectRoot }) {
    super();
    this.projectRoot = projectRoot;
    this.child = null;
    this.nextRequestId = 1;
    this.pending = new Map();
    this.stdoutBuffer = "";
  }

  request(method, params = {}) {
    const child = this.ensureStarted();
    const id = this.nextRequestId++;
    const payload = JSON.stringify({
      jsonrpc: "2.0",
      id,
      method,
      params,
    });

    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      child.stdin.write(`${payload}\n`, (error) => {
        if (error) {
          this.pending.delete(id);
          reject(error);
        }
      });
    });
  }

  ensureStarted() {
    if (this.child && !this.child.killed) {
      return this.child;
    }

    const { executable, args } = backendArgs("backend-daemon", this.projectRoot);
    const child = spawn(executable, args, {
      cwd: this.projectRoot,
      env: process.env,
      stdio: ["pipe", "pipe", "pipe"],
    });

    this.child = child;
    this.stdoutBuffer = "";

    child.stdout.on("data", (chunk) => {
      this.stdoutBuffer += chunk.toString();
      const lines = this.stdoutBuffer.split(/\r?\n/);
      this.stdoutBuffer = lines.pop() || "";
      for (const line of lines) {
        this.handleLine(line);
      }
    });

    child.stderr.on("data", (chunk) => {
      const text = chunk.toString().trim();
      if (text) {
        console.warn(`[backend] ${text}`);
      }
    });

    child.on("close", () => {
      if (this.child === child) {
        this.child = null;
      }
      this.rejectPending(new Error("backend daemon exited"));
    });

    child.on("error", (error) => {
      this.rejectPending(error);
    });

    return child;
  }

  handleLine(line) {
    const trimmed = line.trim();
    if (!trimmed) return;

    let message;
    try {
      message = JSON.parse(trimmed);
    } catch (error) {
      console.warn(`[backend] invalid JSON-RPC line: ${error.message}`);
      return;
    }

    if (message.method === "backend/event") {
      this.emit("backend:event", message.params);
      return;
    }

    const pending = this.pending.get(message.id);
    if (!pending) return;
    this.pending.delete(message.id);

    if (message.error) {
      pending.reject(new Error(message.error.message || "backend JSON-RPC request failed"));
      return;
    }
    pending.resolve(message.result);
  }

  rejectPending(error) {
    for (const pending of this.pending.values()) {
      pending.reject(error);
    }
    this.pending.clear();
  }

  shutdown() {
    if (this.child) {
      this.child.kill();
      this.child = null;
    }
    this.rejectPending(new Error("backend daemon stopped"));
  }
}

module.exports = {
  BackendRpcClient,
  backendArgs,
};
