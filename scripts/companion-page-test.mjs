#!/usr/bin/env node
// Tests the phone page (crates/lume-core/assets/companion/index.html) in a headless Chrome driven
// through the DevTools protocol, against a stand-in for the TV that keeps what the page sends.
//
//   cargo build -p lume-core          # makes the page (OUT_DIR/companion_page.html)
//   node scripts/companion-page-test.mjs [--chrome PATH] [--shots DIR] [--heic DIR]
//   node scripts/companion-page-test.mjs --live http://<tv>:<port>/wallpaper/?c=<code> [--heic DIR]
//
// It checks the cases of the specs custom-wallpaper and phone-companion: the starting mode by
// proportion and size, the 608 px picture and 656 px bars of a portrait photo in Fit, Fill at 150%
// dragged to the edge without black, Repeat with copies edge to edge (and mirrored without a seam),
// a 256x256 picture only in Repeat, the zoom limits, pinch and wheel, sending 1920x1080 twice
// without choosing again, the pings (only while visible), and the layout at 320, 360, 820 and
// 1440 px. With --heic DIR, the HEIC photos in it are opened too (the decoder in WebAssembly).
// Needs Node 22 or newer (WebSocket) and Chrome.

import { spawn } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const args = process.argv.slice(2);
const option = (name, fallback) => {
  const at = args.indexOf(name);
  return at >= 0 ? args[at + 1] : fallback;
};
const CHROME = option("--chrome", process.env.CHROME || "google-chrome");
const SHOTS = path.resolve(option("--shots", path.join(root, "target/companion-page")));
const HEIC_DIR = option("--heic", null);
// --live only: after the last send, wait for Apply on the TV and the page's end.
const WAIT_APPLIED = args.includes("--wait-applied");
const CODE = "TEST";

// ---- the page, as the TV serves it ---------------------------------------------------------------
function builtTemplate() {
  const build = path.join(root, "target/debug/build");
  const found = fs.readdirSync(build)
    .filter((d) => d.startsWith("lume-core-"))
    .map((d) => path.join(build, d, "out/companion_page.html"))
    .filter((f) => fs.existsSync(f))
    .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
  if (!found.length) throw new Error("no companion_page.html: run `cargo build -p lume-core` first");
  return fs.readFileSync(found[0], "utf8");
}
function texts(folder) {
  const xml = fs.readFileSync(path.join(root, "android/app/src/main/res", folder, "strings.xml"), "utf8");
  const out = {};
  for (const m of xml.matchAll(/<string name="companion_([a-z_]+)">(.*?)<\/string>/g)) {
    out[m[1]] = m[2].replace(/\\'/g, "'").replace(/\\"/g, '"').replace(/&amp;/g, "&").replace(/&lt;/g, "<");
  }
  return out;
}
const L = texts("values");
const page = builtTemplate()
  .replace("__LUME_LANG__", "en")
  .replace("__LUME_STRINGS__", JSON.stringify(L).replace(/</g, "\\u003c"));

// ---- QOI ---------------------------------------------------------------------------------------
function qoiDecode(buf) {
  if (buf.toString("latin1", 0, 4) !== "qoif") throw new Error("not QOI");
  const w = buf.readUInt32BE(4), h = buf.readUInt32BE(8);
  const rgb = new Uint8Array(w * h * 3);
  const index = new Uint8Array(64 * 4);
  let r = 0, g = 0, b = 0, a = 255, p = 14, run = 0;
  for (let i = 0; i < w * h; i++) {
    if (run > 0) run--;
    else {
      const b1 = buf[p++];
      if (b1 === 0xfe) { r = buf[p++]; g = buf[p++]; b = buf[p++]; }
      else if (b1 === 0xff) { r = buf[p++]; g = buf[p++]; b = buf[p++]; a = buf[p++]; }
      else if ((b1 & 0xc0) === 0x00) { const at = b1 * 4; r = index[at]; g = index[at + 1]; b = index[at + 2]; a = index[at + 3]; }
      else if ((b1 & 0xc0) === 0x40) { r = (r + ((b1 >> 4) & 3) - 2) & 255; g = (g + ((b1 >> 2) & 3) - 2) & 255; b = (b + (b1 & 3) - 2) & 255; }
      else if ((b1 & 0xc0) === 0x80) {
        const b2 = buf[p++], vg = (b1 & 0x3f) - 32;
        r = (r + vg - 8 + ((b2 >> 4) & 15)) & 255; g = (g + vg) & 255; b = (b + vg - 8 + (b2 & 15)) & 255;
      } else run = b1 & 0x3f;
      const at = ((r * 3 + g * 5 + b * 7 + a * 11) % 64) * 4;
      index[at] = r; index[at + 1] = g; index[at + 2] = b; index[at + 3] = a;
    }
    rgb[i * 3] = r; rgb[i * 3 + 1] = g; rgb[i * 3 + 2] = b;
  }
  return { w, h, rgb, px: (x, y) => [...rgb.subarray((y * w + x) * 3, (y * w + x) * 3 + 3)] };
}

// ---- the TV stand-in ---------------------------------------------------------------------------
const tv = { uploads: [], pings: [], lib: [], verdict: "none" };
const server = http.createServer((req, res) => {
  const url = new URL(req.url, "http://tv");
  const json = (status, body) => { res.writeHead(status, { "Content-Type": "application/json" }); res.end(JSON.stringify(body)); };
  if (req.method === "GET" && url.pathname === "/wallpaper/") {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    return res.end(page);
  }
  if (req.method === "GET" && url.pathname.startsWith("/lib/")) {
    const name = url.pathname.slice(5);
    const file = path.join(root, "crates/lume-core/assets/companion/heic", name);
    tv.lib.push(name);
    if (!/^heic\.(js|wasm)$/.test(name) || !fs.existsSync(file)) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { "Content-Type": name.endsWith(".wasm") ? "application/wasm" : "text/javascript" });
    return res.end(fs.readFileSync(file));
  }
  if (req.method === "POST" && url.pathname === "/ping") {
    if (url.searchParams.get("c") !== CODE) return json(403, { ok: false, error: "code" });
    tv.pings.push({ idle: Number(url.searchParams.get("idle")), at: Date.now() });
    return json(200, { ok: true, tv: tv.verdict });
  }
  if (req.method === "POST" && url.pathname === "/wallpaper/upload") {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => {
      const body = Buffer.concat(chunks);
      try {
        const image = qoiDecode(body);
        if (image.w !== 1920 || image.h !== 1080) return json(422, { ok: false, error: "read" });
        if (tv.verdict === "applied") return json(409, { ok: false, error: "applied" });
        tv.uploads.push(image);
        tv.verdict = "pending";
        json(200, { ok: true });
      } catch (e) { json(422, { ok: false, error: "read" }); }
    });
    return;
  }
  res.writeHead(404); res.end();
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const BASE = `http://127.0.0.1:${server.address().port}`;

// ---- Chrome ------------------------------------------------------------------------------------
const profile = fs.mkdtempSync(path.join(os.tmpdir(), "lume-page-test-"));
const chrome = spawn(CHROME, ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
  "--no-first-run", "--no-default-browser-check", "about:blank"], { stdio: ["ignore", "ignore", "pipe"] });
const wsUrl = await new Promise((ok, fail) => {
  let text = "";
  chrome.stderr.on("data", (d) => { text += d; const m = text.match(/ws:\/\/\S+/); if (m) ok(m[0]); });
  chrome.on("exit", () => fail(new Error("Chrome exited: " + text)));
});
const ws = new WebSocket(wsUrl);
await new Promise((ok) => ws.addEventListener("open", ok));
let nextId = 1;
const waiting = new Map();
const listeners = [];
ws.addEventListener("message", (e) => {
  const msg = JSON.parse(e.data);
  if (msg.id && waiting.has(msg.id)) {
    const { ok, fail } = waiting.get(msg.id);
    waiting.delete(msg.id);
    msg.error ? fail(new Error(JSON.stringify(msg.error))) : ok(msg.result);
  } else if (msg.method) listeners.forEach((l) => l(msg));
});
const cdp = (method, params = {}, sessionId) => new Promise((ok, fail) => {
  const id = nextId++;
  waiting.set(id, { ok, fail });
  ws.send(JSON.stringify({ id, method, params, sessionId }));
});
const { targetId } = await cdp("Target.createTarget", { url: "about:blank" });
const { sessionId } = await cdp("Target.attachToTarget", { targetId, flatten: true });
const send = (method, params) => cdp(method, params, sessionId);
await send("Page.enable");
await send("Runtime.enable");
await send("Page.addScriptToEvaluateOnNewDocument", { source: "window.LUME_PING_MS = 300;" });
const consoleLines = [];
listeners.push((m) => {
  if (m.method === "Runtime.consoleAPICalled") consoleLines.push(m.params.args.map((a) => a.value ?? a.description).join(" "));
  if (m.method === "Runtime.exceptionThrown") consoleLines.push(m.params.exceptionDetails.exception?.description || m.params.exceptionDetails.text);
});

async function evaluate(expression) {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || JSON.stringify(r.exceptionDetails));
  return r.result.value;
}
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
async function until(what, check, ms = 15000) {
  const end = Date.now() + ms;
  for (;;) {
    const v = await check();
    if (v) return v;
    if (Date.now() > end) throw new Error("timed out waiting for " + what);
    await sleep(50);
  }
}
async function viewport(width, height, mobile) {
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: mobile ? 2 : 1, mobile });
  await send("Emulation.setTouchEmulationEnabled", { enabled: mobile, maxTouchPoints: mobile ? 5 : 1 });
}
async function open(query = `?c=${CODE}`) {
  const loaded = new Promise((ok) => {
    const l = (m) => { if (m.method === "Page.loadEventFired") { listeners.splice(listeners.indexOf(l), 1); ok(); } };
    listeners.push(l);
  });
  await send("Page.navigate", { url: `${BASE}/wallpaper/${query}` });
  await loaded;
}

// A made-up picture chosen with the file input. kind: solid (red), edge (a green left edge on blue),
// gradient (red across, green down).
async function choose(w, h, kind = "solid") {
  await evaluate(`(async () => {
    const c = document.createElement("canvas"); c.width = ${w}; c.height = ${h};
    const g = c.getContext("2d");
    if ("${kind}" === "solid") { g.fillStyle = "rgb(220,30,30)"; g.fillRect(0, 0, ${w}, ${h}); }
    else if ("${kind}" === "edge") { g.fillStyle = "rgb(30,30,220)"; g.fillRect(0, 0, ${w}, ${h}); g.fillStyle = "rgb(30,220,30)"; g.fillRect(0, 0, 8, ${h}); }
    else {
      const d = g.createImageData(${w}, ${h});
      for (let y = 0; y < ${h}; y++) for (let x = 0; x < ${w}; x++) {
        const o = (y * ${w} + x) * 4;
        d.data[o] = x * 255 / ${w}; d.data[o + 1] = y * 255 / ${h}; d.data[o + 2] = 100; d.data[o + 3] = 255;
      }
      g.putImageData(d, 0, 0);
    }
    const blob = await new Promise((ok) => c.toBlob(ok, "image/png"));
    const dt = new DataTransfer();
    dt.items.add(new File([blob], "photo.png", { type: "image/png" }));
    const input = document.getElementById("file");
    input.files = dt.files;
    input.dispatchEvent(new Event("change"));
  })()`);
  await ready();
}
async function chooseFile(file) {
  const data = fs.readFileSync(file).toString("base64");
  await evaluate(`(async () => {
    const bytes = Uint8Array.from(atob("${data}"), (c) => c.charCodeAt(0));
    const dt = new DataTransfer();
    dt.items.add(new File([bytes], ${JSON.stringify(path.basename(file))}, { type: "" }));
    const input = document.getElementById("file");
    input.files = dt.files;
    input.dispatchEvent(new Event("change"));
  })()`);
}
const ready = () => until("the editor", () => evaluate(`!document.getElementById("editing").classList.contains("hidden") && document.getElementById("msg").textContent === ""`));
const state = () => evaluate(`(() => {
  const modes = [...document.querySelectorAll("#modes .mode")];
  const mode = modes.find((c) => c.getAttribute("aria-pressed") === "true")?.dataset.mode;
  return {
    mode,
    disabled: modes.filter((c) => c.getAttribute("aria-disabled") === "true").map((c) => c.dataset.mode),
    note: document.getElementById("note").textContent,
    value: document.getElementById(mode === "tile" ? "scale-value" : "zoom-value").textContent,
    send: document.getElementById("send").textContent,
  };
})()`);
// Moves a slider as a finger would (the input event).
const slide = (id, value) => evaluate(`(() => { const r = document.getElementById("${id}"); r.value = ${value}; r.dispatchEvent(new Event("input")); })()`);
const click = (selector) => evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
async function sendToTv() {
  const before = tv.uploads.length;
  await click("#send");
  await until("the upload", () => tv.uploads.length > before);
  await until("the done message", () => evaluate(`document.getElementById("msg").className.includes("done")`));
  return tv.uploads[tv.uploads.length - 1];
}
async function canvasBox() {
  return evaluate(`(() => { const r = document.getElementById("preview").getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; })()`);
}
async function drag(dx, dy) {
  const b = await canvasBox();
  const x = b.x + b.w / 2, y = b.y + b.h / 2;
  await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", clickCount: 1 });
  for (let i = 1; i <= 10; i++) {
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: x + dx * i / 10, y: y + dy * i / 10, button: "left", buttons: 1 });
  }
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: x + dx, y: y + dy, button: "left", clickCount: 1 });
}

let failures = 0;
const results = [];
async function check(name, body) {
  try { await body(); results.push("ok   " + name); }
  catch (e) { failures++; results.push("FAIL " + name + ": " + e.message); }
}
const assert = (cond, message) => { if (!cond) throw new Error(message); };
const near = (a, b, tolerance) => a.every((v, i) => Math.abs(v - b[i]) <= tolerance);
const isBlack = (p) => p.every((v) => v < 12);

// ---- against a real TV (--live URL, the address of its QR) -----------------------------------------
// The TV checks what arrives; here the page must say it was sent, twice, and a HEIC (from --heic)
// must come through the decoder the TV serves from its assets.
const LIVE = option("--live", null);
if (LIVE) {
  const sent = () => until("the TV's answer", () => evaluate(`(() => {
    const m = document.getElementById("msg");
    return m.className.includes("done") ? "done" : m.className.includes("error") ? "error: " + m.textContent : false;
  })()`), 60000);
  await viewport(360, 740, true);
  const loaded = new Promise((ok) => {
    const l = (m) => { if (m.method === "Page.loadEventFired") { listeners.splice(listeners.indexOf(l), 1); ok(); } };
    listeners.push(l);
  });
  await send("Page.navigate", { url: LIVE });
  await loaded;
  await check("live: a portrait photo in Fit", async () => {
    await choose(1080, 1920, "gradient");
    await click("#send");
    const answer = await sent();
    assert(answer === "done", answer);
  });
  await sleep(4000);
  await check("live: the same photo again, in Fill at 130%", async () => {
    await click('#modes .mode[data-mode="cover"]');
    await slide("zoom", 130);
    await click("#send");
    const answer = await sent();
    assert(answer === "done", answer);
  });
  if (HEIC_DIR) {
    const file = fs.readdirSync(HEIC_DIR).find((f) => /\.hei[cf]$/i.test(f));
    await sleep(4000);
    await check(`live: HEIC ${file} through the TV's decoder`, async () => {
      await chooseFile(path.join(HEIC_DIR, file));
      await until("the HEIC photo", () => evaluate(`!document.getElementById("editing").classList.contains("hidden") && document.getElementById("msg").textContent === ""`), 60000);
      await click("#send");
      const answer = await sent();
      assert(answer === "done", answer);
    });
  }
  if (WAIT_APPLIED) {
    await check("live: Apply on the TV ends the page", async () => {
      console.log("press Apply on the TV");
      await until("the page's end", () => evaluate(`!document.getElementById("finished").classList.contains("hidden")`), 120000);
    });
  }
  await finish();
}

// ---- the cases ---------------------------------------------------------------------------------
await viewport(1440, 900, false);
await open();

await check("portrait photo: Fit, 608 px with bars of 656", async () => {
  await choose(1080, 1920);
  const s = await state();
  assert(s.mode === "contain", "mode " + s.mode);
  const out = await sendToTv();
  assert(out.w === 1920 && out.h === 1080, `${out.w}x${out.h}`);
  for (const [x, black] of [[0, true], [655, true], [656, false], [960, false], [1263, false], [1264, true], [1919, true]]) {
    const p = out.px(x, 540);
    assert(black ? isBlack(p) : near(p, [220, 30, 30], 3), `x ${x}: ${p}`);
  }
});

await check("almost 16:9 starts in Fill; panorama in Fit with bars above and below", async () => {
  await choose(2000, 1080);
  assert((await state()).mode === "cover", "2000x1080");
  await choose(3000, 1000);
  assert((await state()).mode === "contain", "3000x1000");
  const out = await sendToTv();
  assert(isBlack(out.px(960, 100)) && isBlack(out.px(960, 980)), "bars");
  assert(near(out.px(960, 540), [220, 30, 30], 3), "the picture in the middle");
});

await check("a 60 MP photo is reduced on the phone and sent", async () => {
  await choose(8000, 7500, "gradient");
  const s = await state();
  assert(s.mode === "contain", "mode " + s.mode);
  const out = await sendToTv();
  assert(out.w === 1920 && out.h === 1080 && !isBlack(out.px(960, 540)), "sent");
});

await check("after choosing, the choice gives way to the editor and 'Change photo'", async () => {
  await open();
  assert(await evaluate(`!document.getElementById("pick").classList.contains("hidden") && document.getElementById("editing").classList.contains("hidden")`), "only the choice at first");
  await choose(1920, 1080, "gradient");
  const m = await evaluate(`(() => {
    const visible = (id) => document.getElementById(id).offsetParent !== null;
    const thumbs = [...document.querySelectorAll("#modes .mode canvas")].map((c) => {
      const d = c.getContext("2d").getImageData(80, 45, 1, 1).data;
      return d[0] + d[1] + d[2];
    });
    return { pick: visible("pick"), replace: visible("replace"), thumbs, desc: document.getElementById("desc").textContent };
  })()`);
  assert(!m.pick && m.replace, JSON.stringify(m));
  assert(m.thumbs.length === 5 && m.thumbs.every((v) => v > 0), "every mode shows the photo: " + m.thumbs);
  assert(m.desc === L.desc_cover, "the sentence of the mode: " + m.desc);
  // "Change photo" opens the file chooser again (the input is the same).
  assert(await evaluate(`document.getElementById("replace").textContent === ${JSON.stringify(L.replace)}`), "label");
});

await check("a locked mode says why", async () => {
  await choose(256, 256, "gradient");
  await click('#modes .mode[data-mode="contain"]');
  const s = await state();
  assert(s.mode === "tile" && s.note === L.tile_only, JSON.stringify(s));
});

await check("Center brings Fill back to the middle", async () => {
  await choose(1920, 1080, "edge");
  await slide("zoom", 150);
  await drag(2000, 0);
  await click("#center");
  const out = await sendToTv();
  assert(!near(out.px(2, 540), [30, 220, 30], 6), "the green edge is out of the screen again");
});

await check("1280x720 starts in Fill with the warning", async () => {
  await choose(1280, 720);
  const s = await state();
  assert(s.mode === "cover" && s.note === L.warn_soft, JSON.stringify(s));
});

await check("256x256 only in Repeat, copies of 256 edge to edge", async () => {
  await choose(256, 256, "gradient");
  const s = await state();
  assert(s.mode === "tile", "mode " + s.mode);
  assert(s.disabled.length === 4 && !s.disabled.includes("tile"), "disabled " + s.disabled);
  assert(s.note === L.tile_only, "note " + s.note);
  const out = await sendToTv();
  for (let x = 0; x < 1920; x += 7) assert(!isBlack(out.px(x, 540)) || out.px(x, 540)[2] > 50, "black at " + x);
  for (let x = 100; x < 1600; x += 37) assert(near(out.px(x, 540), out.px(x + 256, 540), 3), "period at " + x);
});

await check("Repeat mirrored: neighbors meet without a jump", async () => {
  await choose(640, 480, "gradient");
  await click('#variants button:nth-child(2)');
  const out = await sendToTv();
  // The middle copy covers 640..1280; the right one starts mirrored at 1280.
  assert(near(out.px(1279, 300), out.px(1280, 300), 6), `${out.px(1279, 300)} / ${out.px(1280, 300)}`);
  assert(near(out.px(640, 300), out.px(639, 300), 6), "left seam");
  await click('#variants button:nth-child(1)');
  const grid = await sendToTv();
  assert(!near(grid.px(1279, 300), grid.px(1280, 300), 40), "side by side repeats the same copy");
});

await check("Fill at 150% dragged to the edge leaves no black", async () => {
  await choose(1920, 1080, "edge");
  await slide("zoom", 150);
  assert((await state()).value === "150%", "150%");
  await drag(2000, 0);
  await sleep(100);
  const out = await sendToTv();
  assert(near(out.px(2, 540), [30, 220, 30], 6), "the left edge of the photo at the left of the screen: " + out.px(2, 540));
  for (let x = 0; x < 1920; x += 5) for (const y of [0, 540, 1079]) assert(!isBlack(out.px(x, y)), `black at ${x},${y}`);
});

await check("the zoom stops at 200% (buttons and wheel)", async () => {
  await slide("zoom", 260);
  let s = await state();
  assert(s.value === "200%", JSON.stringify(s));
  const b = await canvasBox();
  await send("Input.dispatchMouseEvent", { type: "mouseWheel", x: b.x + 20, y: b.y + 20, deltaX: 0, deltaY: -2000 });
  await sleep(100);
  s = await state();
  assert(s.value === "200%", s.value);
  await send("Input.dispatchMouseEvent", { type: "mouseWheel", x: b.x + 20, y: b.y + 20, deltaX: 0, deltaY: 300 });
  await sleep(100);
  s = await state();
  assert(parseInt(s.value) < 200, "the wheel zooms out: " + s.value);
});

await check("sending again with another zoom, without choosing the photo again", async () => {
  await choose(1920, 1080, "gradient");
  const first = await sendToTv();
  await slide("zoom", 130);
  assert((await state()).send === L.send_again, "the button sends again");
  const second = await sendToTv();
  assert(second.w === 1920 && second.h === 1080, "size");
  assert(!near(first.px(100, 100), second.px(100, 100), 2), "the second framing differs");
});

await check("Apply on the TV ends the page, and its pings stop", async () => {
  await open();
  await choose(1920, 1080, "gradient");
  await sendToTv();
  const before = tv.pings.length;
  tv.verdict = "applied";
  await until("the page's end", () => evaluate(`!document.getElementById("finished").classList.contains("hidden")`), 5000);
  const shown = await evaluate(`(() => ({
    editing: document.getElementById("editing").offsetParent !== null,
    text: document.getElementById("finished").innerText,
  }))()`);
  assert(!shown.editing, "the editor is gone");
  assert(shown.text.includes(L.applied) && shown.text.includes(L.close_tab), shown.text);
  assert(tv.pings.length > before, "it asked the TV");
  const after = tv.pings.length;
  await sleep(2500);
  assert(tv.pings.length === after, "no pings after the end");
  tv.verdict = "none";
});

await check("Cancel on the TV says so and the editing goes on", async () => {
  await open();
  await choose(1920, 1080, "gradient");
  await sendToTv();
  tv.verdict = "declined";
  await until("the message", () => evaluate(`document.getElementById("msg").textContent === ${JSON.stringify(L.declined)}`), 5000);
  assert(await evaluate(`document.getElementById("editing").offsetParent !== null && !document.getElementById("send").disabled`), "editing");
  tv.verdict = "none";
});

await check("pings while visible, none while hidden", async () => {
  await open();
  const start = tv.pings.length;
  await sleep(1200);
  const visible = tv.pings.length - start;
  assert(visible >= 2, visible + " pings");
  await evaluate(`Object.defineProperty(document, "visibilityState", { get: () => "hidden", configurable: true }); document.dispatchEvent(new Event("visibilitychange"));`);
  await sleep(200);
  const hidden = tv.pings.length;
  await sleep(1200);
  assert(tv.pings.length === hidden, (tv.pings.length - hidden) + " pings while hidden");
  const idle = tv.pings[tv.pings.length - 1].idle;
  assert(idle >= 0 && idle < 120, "idle " + idle);
});

await check("a wrong code makes the page ask for it", async () => {
  await open("?c=ZZZZ");
  await until("the code panel", () => evaluate(`!document.getElementById("code-panel").classList.contains("hidden")`), 3000);
  await open("");
  assert(await evaluate(`!document.getElementById("code-panel").classList.contains("hidden")`), "typed by hand");
});

fs.mkdirSync(SHOTS, { recursive: true });
for (const [width, height, mobile] of [[320, 640, true], [360, 740, true], [820, 1180, true], [1440, 900, false]]) {
  await check(`layout at ${width} px`, async () => {
    await viewport(width, height, mobile);
    await open();
    const empty = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
    fs.writeFileSync(path.join(SHOTS, `page-${width}-empty.png`), Buffer.from(empty.data, "base64"));
    await choose(2400, 1350, "gradient");
    const m = await evaluate(`(() => {
      const visible = (el) => el.offsetParent !== null && el.getClientRects().length;
      const buttons = [...document.querySelectorAll("button, label.button")].filter(visible);
      const texts = [...document.querySelectorAll("body *")].filter((el) => visible(el) && [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()));
      const l = document.getElementById("left").getBoundingClientRect(), r = document.getElementById("right").getBoundingClientRect();
      return {
        overflow: document.documentElement.scrollWidth - window.innerWidth,
        smallestButton: Math.min(...buttons.map((b) => b.getBoundingClientRect().height)),
        smallestText: Math.min(...texts.map((t) => parseFloat(getComputedStyle(t).fontSize))),
        main: document.querySelector("main").getBoundingClientRect().width,
        sideBySide: r.left >= l.right,
      };
    })()`);
    assert(m.overflow <= 0, "horizontal scroll of " + m.overflow);
    assert(m.smallestButton >= 48, "a button of " + m.smallestButton);
    assert(m.smallestText >= 16, "text of " + m.smallestText);
    if (width === 820) assert(m.main <= 640, "column of " + m.main);
    if (width === 1440) assert(m.sideBySide, "two columns");
    const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true });
    fs.writeFileSync(path.join(SHOTS, `page-${width}.png`), Buffer.from(shot.data, "base64"));
  });
}

await check("pinch zooms Fill, and turning the phone keeps the photo and the zoom", async () => {
  await viewport(360, 740, true);
  await open();
  await choose(1920, 1080);
  const b = await canvasBox();
  const cx = b.x + b.w / 2, cy = b.y + b.h / 2;
  const points = (d) => [{ x: cx - d, y: cy, id: 1 }, { x: cx + d, y: cy, id: 2 }];
  await send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: points(20) });
  for (let d = 25; d <= 60; d += 5) await send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: points(d) });
  await send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await sleep(100);
  const zoom = (await state()).value;
  assert(parseInt(zoom) > 100, "pinch: " + zoom);
  const scrolled = await evaluate("window.scrollY");
  assert(scrolled === 0, "the page scrolled by " + scrolled);
  await viewport(740, 360, true);
  await sleep(200);
  const s = await state();
  assert(s.value === zoom && s.mode === "cover", JSON.stringify(s));
});

await check("a JPEG or PNG never loads the HEIC decoder", async () => {
  assert(!tv.lib.length, "asked for " + tv.lib);
});

if (HEIC_DIR) {
  for (const file of fs.readdirSync(HEIC_DIR).filter((f) => /\.hei[cf]$/i.test(f))) {
    await check(`HEIC ${file}`, async () => {
      await viewport(1440, 900, false);
      await open();
      await chooseFile(path.join(HEIC_DIR, file));
      const outcome = await until("the HEIC photo", () => evaluate(`(() => {
        const msg = document.getElementById("msg");
        if (msg.className.includes("error")) return "error: " + msg.textContent;
        return !document.getElementById("editing").classList.contains("hidden") && msg.textContent === "" && "ok";
      })()`), 60000);
      assert(outcome === "ok", outcome + " (asked the TV for " + tv.lib.join(", ") + "; console: " + consoleLines.slice(-3).join(" | ") + ")");
      const size = await evaluate(`(() => { const s = document.getElementById("preview"); return [s.width, s.height]; })()`);
      const out = await sendToTv();
      assert(out.w === 1920, "sent");
      const expected = path.join(HEIC_DIR, file.replace(/\.[^.]+$/, ".json"));
      if (fs.existsSync(expected)) {
        // The orientation: {"portrait": true} for a photo taken upright. In Fit, a 3:4 photo is 810
        // px wide (bars of 555), a 4:3 one 1440 (bars of 240): x = 400 tells them apart.
        const want = JSON.parse(fs.readFileSync(expected, "utf8"));
        const mode = (await state()).mode;
        assert(mode === "contain", "mode " + mode + " " + size);
        assert(!isBlack(out.px(960, 540)), "the photo in the middle");
        assert(isBlack(out.px(400, 540)) === !!want.portrait, (want.portrait ? "portrait" : "landscape") + " expected: " + out.px(400, 540));
      }
      assert(tv.lib.includes("heic.wasm"), "the decoder was not loaded");
    });
  }
}

await finish();

async function finish() {
  for (const line of results) console.log(line);
  console.log(failures ? `${failures} failed` : "all passed");
  ws.close();
  const exited = new Promise((ok) => chrome.on("exit", ok));
  chrome.kill();
  await exited;
  server.close();
  fs.rmSync(profile, { recursive: true, force: true });
  process.exit(failures ? 1 : 0);
}
