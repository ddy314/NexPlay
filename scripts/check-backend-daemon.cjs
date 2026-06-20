const { spawn } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "nexplay-backend-daemon-"));
const configPath = path.join(tempRoot, "config.toml");
const databasePath = path.join(tempRoot, "nexplay.sqlite3");

fs.writeFileSync(configPath, `
media_libraries = []

[database]
path = "${databasePath.replaceAll("\\", "\\\\")}"

[dandanplay]
app_id = ""
app_secret = ""
api_key = ""

[bangumi]
enabled = true
base_url = "https://api.bgm.tv"
access_token = ""
user_agent = "NexPlay/0.1.0"
request_timeout_secs = 20
auto_match = true
cache_images = true

[logging]
level = "info"
`);

const child = spawn("cargo", ["run", "--quiet", "--", "backend-daemon"], {
  cwd: projectRoot,
  env: {
    ...process.env,
    NEXPLAY_CONFIG: configPath,
  },
  stdio: ["pipe", "pipe", "pipe"],
});

let nextId = 1;
let stdoutBuffer = "";
const pending = new Map();
const notifications = [];

child.stdout.on("data", (chunk) => {
  stdoutBuffer += chunk.toString();
  const lines = stdoutBuffer.split(/\r?\n/);
  stdoutBuffer = lines.pop() || "";
  for (const line of lines) {
    if (!line.trim()) continue;
    const message = JSON.parse(line);
    if (message.method === "backend/event") {
      notifications.push(message.params);
      continue;
    }
    const request = pending.get(message.id);
    if (!request) continue;
    pending.delete(message.id);
    message.error ? request.reject(new Error(message.error.message)) : request.resolve(message.result);
  }
});

child.stderr.on("data", (chunk) => {
  const text = chunk.toString().trim();
  if (text) {
    console.warn(`[backend-daemon] ${text}`);
  }
});

function request(method, params = {}) {
  const id = nextId++;
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`timeout waiting for ${method}`));
    }, 20000);
    pending.set(id, {
      resolve: (value) => {
        clearTimeout(timeout);
        resolve(value);
      },
      reject: (error) => {
        clearTimeout(timeout);
        reject(error);
      },
    });
  });
}

async function requestError(method, params = {}) {
  try {
    await request(method, params);
  } catch (error) {
    return error.message;
  }
  throw new Error(`${method} unexpectedly succeeded`);
}

async function main() {
  const pid = child.pid;
  const snapshot = await request("snapshot");
  const settings = await request("getSettings");
  const danmakuError = await requestError("danmakuTrack", { mediaId: 999 });
  if (child.pid !== pid) {
    throw new Error("backend daemon process changed between requests");
  }
  if (!snapshot?.stats || !settings?.databasePath) {
    throw new Error("backend daemon returned invalid payloads");
  }
  if (!danmakuError.includes("selected media item was not found")) {
    throw new Error(`unexpected danmakuTrack error: ${danmakuError}`);
  }
  console.log(JSON.stringify({
    ok: true,
    pid,
    requests: 3,
    notifications: notifications.length,
    databasePath: settings.databasePath,
  }, null, 2));
}

main()
  .catch((error) => {
    console.error(JSON.stringify({ ok: false, error: error.message }, null, 2));
    process.exitCode = 1;
  })
  .finally(() => {
    child.kill();
    fs.rmSync(tempRoot, { recursive: true, force: true });
  });
