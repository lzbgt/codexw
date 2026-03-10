#!/usr/bin/env node

import http from "node:http";
import https from "node:https";
import process from "node:process";

function fail(message) {
  console.error(message);
  process.exit(2);
}

function parseJson(value, label) {
  try {
    return JSON.parse(value);
  } catch (error) {
    fail(`${label} must be valid JSON: ${error.message}`);
  }
}

function parseArgs(argv) {
  const global = {
    baseUrl: "http://127.0.0.1:4317",
    agentId: "codexw-lab",
    clientId: null,
    leaseSeconds: null,
  };

  let index = 0;
  while (index < argv.length && argv[index].startsWith("--")) {
    const flag = argv[index++];
    const value = argv[index++];
    if (value === undefined) {
      fail(`missing value for ${flag}`);
    }
    switch (flag) {
      case "--base-url":
        global.baseUrl = value;
        break;
      case "--agent-id":
        global.agentId = value;
        break;
      case "--client-id":
        global.clientId = value;
        break;
      case "--lease-seconds":
        global.leaseSeconds = Number.parseInt(value, 10);
        if (!Number.isFinite(global.leaseSeconds)) {
          fail("--lease-seconds must be an integer");
        }
        break;
      default:
        fail(`unknown global option: ${flag}`);
    }
  }

  const command = argv[index++];
  if (!command) {
    fail("missing command");
  }

  const commandArgs = {};
  while (index < argv.length) {
    const flag = argv[index++];
    if (!flag.startsWith("--")) {
      fail(`unexpected positional argument: ${flag}`);
    }
    const key = flag.slice(2).replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    const value = argv[index++];
    if (value === undefined) {
      fail(`missing value for ${flag}`);
    }
    commandArgs[key] = value;
  }

  return { global, command, commandArgs };
}

class BrokerClient {
  constructor({ baseUrl, agentId, clientId, leaseSeconds }) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.agentId = agentId;
    this.clientId = clientId;
    this.leaseSeconds = leaseSeconds;
  }

  url(path) {
    return `${this.baseUrl}/v1/agents/${this.agentId}${path}`;
  }

  requestModule(url) {
    return url.protocol === "https:" ? https : http;
  }

  headers(extra = {}) {
    const headers = {
      Accept: "application/json",
      Connection: "close",
      ...extra,
    };
    if (this.clientId) {
      headers["X-Codexw-Client-Id"] = this.clientId;
    }
    if (this.leaseSeconds !== null && this.leaseSeconds !== undefined) {
      headers["X-Codexw-Lease-Seconds"] = String(this.leaseSeconds);
    }
    return headers;
  }

  async request(method, path, payload = undefined, extraHeaders = {}) {
    const url = new URL(this.url(path));
    const headers = this.headers(extraHeaders);
    let requestBody = null;
    if (payload !== undefined) {
      requestBody = JSON.stringify(payload);
      headers["Content-Type"] = "application/json";
      headers["Content-Length"] = Buffer.byteLength(requestBody).toString();
    }

    return await new Promise((resolve, reject) => {
      const request = this.requestModule(url).request(
        {
          protocol: url.protocol,
          hostname: url.hostname,
          port: url.port,
          path: `${url.pathname}${url.search}`,
          method,
          headers,
        },
        (response) => {
          const chunks = [];
          response.on("data", (chunk) => chunks.push(Buffer.from(chunk)));
          response.on("end", () => {
            const text = Buffer.concat(chunks).toString("utf8");
            let body = null;
            if (text.length > 0) {
              try {
                body = JSON.parse(text);
              } catch {
                body = text;
              }
            }
            resolve({ status: response.statusCode ?? 0, body });
          });
        },
      );
      request.on("error", reject);
      if (requestBody !== null) {
        request.write(requestBody);
      }
      request.end();
    });
  }

  async get(path) {
    return this.request("GET", path);
  }

  async post(path, payload = {}) {
    return this.request("POST", path, payload);
  }

  async streamEvents(sessionId, lastEventId = null, limit = 5) {
    const headers = this.headers({ Accept: "text/event-stream" });
    if (lastEventId !== null && lastEventId !== undefined) {
      headers["Last-Event-ID"] = lastEventId;
    }
    const url = new URL(this.url(`/sessions/${sessionId}/events`));
    return await new Promise((resolve, reject) => {
      const request = this.requestModule(url).request(
        {
          protocol: url.protocol,
          hostname: url.hostname,
          port: url.port,
          path: `${url.pathname}${url.search}`,
          method: "GET",
          headers,
        },
        (response) => {
          const status = response.statusCode ?? 0;
          if (status < 200 || status >= 300) {
            const chunks = [];
            response.on("data", (chunk) => chunks.push(Buffer.from(chunk)));
            response.on("end", () => {
              const text = Buffer.concat(chunks).toString("utf8");
              let body = null;
              if (text.length > 0) {
                try {
                  body = JSON.parse(text);
                } catch {
                  body = text;
                }
              }
              resolve({ status, body });
            });
            return;
          }

          let buffer = "";
          let current = {};
          const items = [];

          const parseBuffer = () => {
            while (items.length < limit) {
              const lineEnd = buffer.indexOf("\n");
              if (lineEnd === -1) {
                break;
              }
              const rawLine = buffer.slice(0, lineEnd);
              buffer = buffer.slice(lineEnd + 1);
              const line = rawLine.replace(/\r$/, "");
              if (line.length === 0) {
                if (Object.keys(current).length > 0) {
                  const item = {};
                  if (current.id !== undefined) {
                    item.id = current.id;
                  }
                  if (current.event !== undefined) {
                    item.event = current.event;
                  }
                  if (current.data !== undefined) {
                    try {
                      item.data = JSON.parse(current.data);
                    } catch {
                      item.data = current.data;
                    }
                  }
                  items.push(item);
                  current = {};
                }
                continue;
              }
              if (line.startsWith(":")) {
                continue;
              }
              const separator = line.indexOf(":");
              if (separator === -1) {
                continue;
              }
              const field = line.slice(0, separator);
              const valueText = line.slice(separator + 1).replace(/^ /, "");
              current[field] = valueText;
            }
          };

          response.setEncoding("utf8");
          response.on("data", (chunk) => {
            if (items.length >= limit) {
              return;
            }
            buffer += chunk;
            parseBuffer();
            if (items.length >= limit) {
              response.destroy();
            }
          });
          response.on("close", () => resolve({ status, body: items }));
          response.on("end", () => resolve({ status, body: items }));
        },
      );
      request.on("error", reject);
      request.end();
    });
  }
}

function requireArg(args, name) {
  const value = args[name];
  if (value === undefined) {
    fail(`missing required option --${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`);
  }
  return value;
}

function maybeArg(args, name) {
  return args[name];
}

async function main() {
  const { global, command, commandArgs } = parseArgs(process.argv.slice(2));
  const client = new BrokerClient(global);
  let result;

  switch (command) {
    case "session-create":
      result = await client.post("/sessions", {
        thread_id: requireArg(commandArgs, "threadId"),
      });
      break;
    case "session-attach":
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/attach`,
        { thread_id: requireArg(commandArgs, "threadId") },
      );
      break;
    case "attachment-renew":
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/attachment/renew`,
        { lease_seconds: Number.parseInt(requireArg(commandArgs, "leaseSeconds"), 10) },
      );
      break;
    case "attachment-release":
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/attachment/release`,
        {},
      );
      break;
    case "sessions":
      result = await client.get("/sessions");
      break;
    case "session-get":
      result = await client.get(`/sessions/${requireArg(commandArgs, "sessionId")}`);
      break;
    case "turn-start":
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/turns`,
        { prompt: requireArg(commandArgs, "prompt") },
      );
      break;
    case "turn-interrupt":
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/interrupt`,
        {},
      );
      break;
    case "client-event": {
      const body = { event: requireArg(commandArgs, "event") };
      const dataJson = maybeArg(commandArgs, "dataJson");
      if (dataJson !== undefined) {
        body.data = parseJson(dataJson, "--data-json");
      }
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/client-events`,
        body,
      );
      break;
    }
    case "events":
      result = await client.streamEvents(
        requireArg(commandArgs, "sessionId"),
        maybeArg(commandArgs, "lastEventId"),
        maybeArg(commandArgs, "limit") ? Number.parseInt(commandArgs.limit, 10) : 5,
      );
      break;
    case "transcript":
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/transcript`,
      );
      break;
    case "orchestration-status":
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/orchestration/status`,
      );
      break;
    case "orchestration-workers":
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/orchestration/workers`,
      );
      break;
    case "orchestration-dependencies":
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/orchestration/dependencies`,
      );
      break;
    case "shells":
      result = await client.get(`/sessions/${requireArg(commandArgs, "sessionId")}/shells`);
      break;
    case "shell-start": {
      const body = { command: requireArg(commandArgs, "shellCommand") };
      if (maybeArg(commandArgs, "intent") !== undefined) {
        body.intent = commandArgs.intent;
      }
      if (maybeArg(commandArgs, "label") !== undefined) {
        body.label = commandArgs.label;
      }
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/shells`,
        body,
      );
      break;
    }
    case "shell-detail": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/shells/${ref}`,
      );
      break;
    }
    case "shell-poll": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/shells/${ref}/poll`,
        {},
      );
      break;
    }
    case "shell-send": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/shells/${ref}/send`,
        { text: requireArg(commandArgs, "text") },
      );
      break;
    }
    case "shell-terminate": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/shells/${ref}/terminate`,
        {},
      );
      break;
    }
    case "services":
      result = await client.get(`/sessions/${requireArg(commandArgs, "sessionId")}/services`);
      break;
    case "service-detail": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}`,
      );
      break;
    }
    case "service-attach": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}/attach`,
        {},
      );
      break;
    }
    case "service-wait": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      const body = {};
      if (maybeArg(commandArgs, "timeoutMs") !== undefined) {
        body.timeout_ms = Number.parseInt(commandArgs.timeoutMs, 10);
      }
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}/wait`,
        body,
      );
      break;
    }
    case "service-run": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      const body = { recipe: requireArg(commandArgs, "recipe") };
      if (maybeArg(commandArgs, "argsJson") !== undefined) {
        body.args = parseJson(commandArgs.argsJson, "--args-json");
      }
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}/run`,
        body,
      );
      break;
    }
    case "service-provide": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}/provide`,
        { capabilities: parseJson(requireArg(commandArgs, "valuesJson"), "--values-json") },
      );
      break;
    }
    case "service-depend": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}/depend`,
        {
          dependsOnCapabilities: parseJson(
            requireArg(commandArgs, "valuesJson"),
            "--values-json",
          ),
        },
      );
      break;
    }
    case "service-contract": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      const contract = parseJson(requireArg(commandArgs, "contractJson"), "--contract-json");
      if (contract === null || Array.isArray(contract) || typeof contract !== "object") {
        fail("--contract-json must decode to an object");
      }
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}/contract`,
        contract,
      );
      break;
    }
    case "service-relabel": {
      const ref = encodeURIComponent(requireArg(commandArgs, "jobRef"));
      const label = requireArg(commandArgs, "label");
      result = await client.post(
        `/sessions/${requireArg(commandArgs, "sessionId")}/services/${ref}/relabel`,
        { label: label === "none" ? null : label },
      );
      break;
    }
    case "capabilities":
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/capabilities`,
      );
      break;
    case "capability-detail": {
      const ref = encodeURIComponent(requireArg(commandArgs, "capability"));
      result = await client.get(
        `/sessions/${requireArg(commandArgs, "sessionId")}/capabilities/${ref}`,
      );
      break;
    }
    default:
      fail(`unsupported command: ${command}`);
  }

  console.log(JSON.stringify(result, null, 2));
}

await main();
