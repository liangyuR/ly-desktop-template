import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = fileURLToPath(new URL("../", import.meta.url));
const { values } = parseArgs({ options: {
  name: { type: "string" }, package: { type: "string" }, identifier: { type: "string" },
  icon: { type: "string" }, "dry-run": { type: "boolean" }, help: { type: "boolean" },
} });
const usage = "node scripts/init-product.mjs --name <产品名> --package <包名> --identifier <反向域名> [--icon <方形 PNG>] [--dry-run]";
const fail = message => { throw new Error(message); };

function checkFile(relative) {
  let absolute = root;
  for (const part of relative.split("/")) {
    absolute = path.join(absolute, part);
    if (lstatSync(absolute).isSymbolicLink()) fail("目标文件或目录是链接：" + relative);
  }
  return absolute;
}

function renamePackage(text, table, oldName, newName) {
  const start = table === "package" ? "[package]" : "[[package]]";
  const blocks = text.split(/(?=^\[)/m);
  const sections = blocks.filter(b => b.split(/\r?\n/, 1)[0] === start);
  const nameOf = s => /^name\s*=\s*"([^"]+)"/m.exec(s)?.[1];
  const sourceOf = s => /^source\s*=/m.test(s);
  if (oldName !== newName && sections.some(s => nameOf(s) === newName))
    fail("新包名已被 Cargo 依赖占用：" + newName);
  const local = sections.filter(s => nameOf(s) === oldName && !sourceOf(s));
  if (local.length !== 1) fail("无法唯一定位 Cargo 包：" + oldName);
  return blocks.map(s => s === local[0] ? s.replace(/^(name\s*=\s*)"[^"]+"/m, '$1"' + newName + '"') : s).join("");
}

function plan() {
  const { name, package: pkg, identifier } = values;
  if (!name || !name.trim() || name !== name.trim() || name.length > 100 ||
      /[<>:"/\\|?*\u0000-\u001f]/u.test(name) || /[ .]$/.test(name)) fail("产品名须为合法的 Windows 显示名称。");
  if (!pkg || !/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/.test(pkg) || pkg.length > 64 ||
      /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i.test(pkg)) fail("包名使用小写字母、数字与连字符。");
  if (!identifier || !/^(?:[a-z][a-z0-9-]*\.)+[a-z][a-z0-9-]*$/.test(identifier) ||
      identifier.length > 150 || identifier === "com.liangyur.lydesktoptemplate")
    fail("请为派生产品指定独立的反向域名标识，例如 com.example.inspector。");

  const changes = [];
  const edit = (relative, transform) => {
    const filename = checkFile(relative), before = readFileSync(filename);
    const after = Buffer.from(transform(before));
    if (!before.equals(after)) changes.push({ relative, filename, before, after });
  };
  const json = (relative, update) => edit(relative, b => {
    const data = JSON.parse(b);
    update(data);
    return JSON.stringify(data, null, 2) + "\n";
  });
  let oldPackage;
  json("package.json", p => { oldPackage = p.name; p.name = pkg; });
  for (const [file, kind] of [["Cargo.toml", "package"], ["Cargo.lock", "lock"]]) {
    edit("src-tauri/" + file, b => renamePackage(b.toString(), kind, oldPackage, pkg));
  }
  json("src-tauri/tauri.conf.json", c => {
    const main = c.app?.windows?.find(w => w.label === "main");
    if (!main) fail("未找到 Tauri 主窗口。");
    c.productName = name;
    c.identifier = identifier;
    main.title = name;
  });
  edit("index.html", b => {
    const html = b.toString();
    if ([...html.matchAll(/<title>[^<]*<\/title>/gi)].length !== 1) fail("无法唯一定位网页标题。");
    return html.replace(/<title>[^<]*<\/title>/i, () => "<title>" + name.replaceAll("&","&amp;") + "</title>");
  });

  if (values.icon) {
    const input = path.resolve(values.icon);
    if (path.extname(input).toLowerCase() !== ".png") fail("图标输入须为方形 PNG。");
    const png = readFileSync(input);
    if (png.length < 24 || !png.subarray(0,8).equals(Buffer.from("89504e470d0a1a0a", "hex")) ||
        png.readUInt32BE(16) !== png.readUInt32BE(20)) fail("图标输入须为方形 PNG。");
    const dir = mkdtempSync(path.join(tmpdir(), "ly-template-icon-"));
    try {
      const require = createRequire(import.meta.url);
      const cli = path.join(path.dirname(require.resolve("@tauri-apps/cli/package.json")), "tauri.js");
      const child = spawnSync(process.execPath, [cli,"icon",input,"--output",dir], { cwd:root, encoding:"utf8", windowsHide:true });
      if (child.error || child.status !== 0) fail("生成图标失败：" + (child.error?.message || child.stderr));
      for (const [file, generated] of [["src-tauri/icons/icon.ico","icon.ico"], ["src-tauri/icons/icon.png","icon.png"], ["public/app-icon.png","icon.png"]]) {
        edit(file, () => readFileSync(path.join(dir, generated)));
      }
    } finally { rmSync(dir,{ recursive:true, force:true }); }
  }
  return changes;
}

function apply(changes) {
  const backup = path.join(root,".ly-product-init");
  mkdirSync(backup);
  const applied = [];
  let keepBackup = false;
  try {
    for (const [index, change] of changes.entries()) {
      const saved = path.join(backup,"original",change.relative);
      mkdirSync(path.dirname(saved), { recursive:true });
      writeFileSync(saved,change.before, { flag:"wx" });
      writeFileSync(path.join(backup,"new-" + index),change.after, { flag:"wx" });
    }
    for (const [index, change] of changes.entries()) {
      if (!readFileSync(checkFile(change.relative)).equals(change.before)) fail("文件在初始化期间被修改：" + change.relative);
      renameSync(path.join(backup,"new-" + index), change.filename);
      applied.push(change);
    }
  } catch (error) {
    for (const change of applied.reverse()) {
      try {
        if (!readFileSync(checkFile(change.relative)).equals(change.after)) fail("文件已再次修改");
        renameSync(path.join(backup,"original",change.relative),change.filename);
      } catch {
        keepBackup = true;
      }
    }
    if (keepBackup) console.error("有文件无法自动回滚，原文保存在 " + backup);
    throw error;
  } finally {
    if (!keepBackup) rmSync(backup,{ recursive:true, force:true });
  }
}

try {
  if (values.help) {
    console.log(usage);
  } else {
    if (existsSync(path.join(root, ".ly-product-init"))) fail("请先核对上次初始化留下的 .ly-product-init/original 备份。");
    const changes = plan();
    if (changes.length) {
      for (const { relative } of changes) console.log(relative);
      if (values["dry-run"]) console.log("预览完成。");
      else { apply(changes); console.log("产品初始化完成。"); }
    } else console.log("配置与目标一致。");
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
