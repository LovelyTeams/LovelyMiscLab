// Independent Node crypto oracle; do not send the answer field to the model.
import { readFile, writeFile } from "node:fs/promises";
import { randomBytes, createCipheriv } from "node:crypto";
import { resolve } from "node:path";
const path = resolve(process.argv[2] ?? ".git/solver-fixtures/cases.json");
const cases = JSON.parse(await readFile(path, "utf8")).filter((c) => c.name !== "aes");
const key = randomBytes(16), iv = randomBytes(16), answer = `flag{aes_${randomBytes(5).toString("hex")}}`;
const cipher = createCipheriv("aes-128-cbc", key, iv);
const data = Buffer.concat([cipher.update(answer, "utf8"), cipher.final()]).toString("hex");
cases.push({ name: "aes", task: `现代密码题，题目提示 AES-CBC、PKCS7。密钥(Hex)：${key.toString("hex")}，IV(Hex)：${iv.toString("hex")}。请配置节点解出答案。`, answer, data, files: [] });
await writeFile(path, JSON.stringify(cases, null, 2), "utf8");
console.log("Added independent AES-CBC fixture with held-out answer.");
