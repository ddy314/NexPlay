const { spawn } = require("node:child_process");
const readline = require("node:readline");

const electronBinary = require("electron");
const child = spawn(electronBinary, process.argv.slice(2), {
  env: process.env,
  stdio: ["inherit", "inherit", "pipe"],
});

const stderr = readline.createInterface({ input: child.stderr });
stderr.on("line", (line) => {
  if (!line.includes("Fontconfig warning:")) {
    process.stderr.write(`${line}\n`);
  }
});

for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"]) {
  process.on(signal, () => child.kill(signal));
}

child.on("error", (error) => {
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});

child.on("close", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exitCode = code ?? 1;
});
