import { useState } from "react";
import { createRoot } from "react-dom/client";
import "@/index.css";
import { SettingsView } from "@/views/SettingsView";
import { api } from "@/lib/bindings";

// UI regression only: no credentials, network model calls or real settings writes.
let failure: "load" | "save" | null = null;
let savedCount = 0;
api.getSettings = async () => {
  if (failure === "load") throw new Error("fixture read failure");
  return { ai: { llm: { model: "fixture", apiKey: "", baseUrl: "http://localhost/v1" }, vision: { model: "", apiKey: "", baseUrl: "" } }, outputDir: "", tools: {} };
};
api.setSettings = async () => {
  if (failure === "save") throw new Error("fixture permission denied");
  savedCount++;
};
function Harness() {
  const [version, setVersion] = useState(0);
  return <main className="flex h-screen flex-col bg-background text-foreground">
    <div className="flex gap-5 border-b p-3 text-xs">
      <strong>设置界面回归 · 不写入真实配置</strong>
      <button onClick={() => { failure = "save"; setVersion(version + 1); }}>模拟保存失败</button>
      <button onClick={() => { failure = "load"; setVersion(version + 1); }}>模拟读取失败</button>
      <button onClick={() => { failure = null; setVersion(version + 1); }}>正常保存</button>
      <button onClick={() => setVersion(version + 1)}>刷新记录</button>
      <span>成功写入 {savedCount} 次</span>
    </div>
    <div className="min-h-0 flex-1"><SettingsView key={version} /></div>
  </main>;
}
createRoot(document.getElementById("root")!).render(<Harness />);
