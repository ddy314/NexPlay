const fs = require("node:fs");
const path = require("node:path");
const { Readable } = require("node:stream");
const { protocol } = require("electron");

function registerAssetProtocol() {
  protocol.handle("nexplay-asset", async (request) => {
    const url = new URL(request.url);
    if (url.hostname !== "local") {
      return new Response("unsupported asset host", { status: 400 });
    }

    const filePath = decodeURIComponent(url.pathname.slice(1));
    return streamLocalFile(filePath, request);
  });
}

function streamLocalFile(filePath, request) {
  let stat;
  try {
    stat = fs.statSync(filePath);
  } catch {
    return new Response("asset not found", { status: 404 });
  }

  if (!stat.isFile()) {
    return new Response("asset is not a file", { status: 404 });
  }

  const range = request.headers.get("range");
  const contentType = contentTypeForPath(filePath);
  const baseHeaders = {
    "Content-Type": contentType,
    "Accept-Ranges": "bytes",
    "Cache-Control": "no-store",
  };

  if (range) {
    const match = /^bytes=(\d*)-(\d*)$/.exec(range);
    if (!match) {
      return new Response("invalid range", {
        status: 416,
        headers: {
          ...baseHeaders,
          "Content-Range": `bytes */${stat.size}`,
        },
      });
    }

    const start = match[1] ? Number(match[1]) : 0;
    const end = match[2] ? Math.min(Number(match[2]), stat.size - 1) : stat.size - 1;
    if (!Number.isFinite(start) || !Number.isFinite(end) || start > end || start >= stat.size) {
      return new Response("range not satisfiable", {
        status: 416,
        headers: {
          ...baseHeaders,
          "Content-Range": `bytes */${stat.size}`,
        },
      });
    }

    const stream = fs.createReadStream(filePath, { start, end });
    return new Response(Readable.toWeb(stream), {
      status: 206,
      headers: {
        ...baseHeaders,
        "Content-Length": String(end - start + 1),
        "Content-Range": `bytes ${start}-${end}/${stat.size}`,
      },
    });
  }

  const stream = fs.createReadStream(filePath);
  return new Response(Readable.toWeb(stream), {
    status: 200,
    headers: {
      ...baseHeaders,
      "Content-Length": String(stat.size),
    },
  });
}

function contentTypeForPath(filePath) {
  switch (path.extname(filePath).toLowerCase()) {
    case ".jpg":
    case ".jpeg":
      return "image/jpeg";
    case ".png":
      return "image/png";
    case ".webp":
      return "image/webp";
    case ".gif":
      return "image/gif";
    case ".mp4":
    case ".m4v":
      return "video/mp4";
    case ".webm":
      return "video/webm";
    case ".mkv":
      return "video/x-matroska";
    case ".mov":
      return "video/quicktime";
    case ".avi":
      return "video/x-msvideo";
    default:
      return "application/octet-stream";
  }
}

module.exports = {
  registerAssetProtocol,
};
