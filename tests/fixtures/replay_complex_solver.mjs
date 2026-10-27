// Re-run the UI-exported graph with the real Rust executor, without an LLM call.
import fs from 'node:fs';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import assert from 'node:assert/strict';

const [projectPath = '.git/complex-solver-final-ui.json', oraclePath = '.git/solver-fixtures/complex/cases.json', evidencePath = '.git/complex-solver-replay.json'] = process.argv.slice(2);
const exported = JSON.parse(fs.readFileSync(projectPath, 'utf8'));
const project = exported.project ?? exported;
const oracle = JSON.parse(fs.readFileSync(oraclePath, 'utf8'));
const build = fs.readFileSync('.git/live-ai-build.jsonl', 'utf8').trim().split('\n').map(JSON.parse)
  .findLast((r) => r.reason === 'compiler-artifact' && r.profile.test && r.executable);
const graph = {
  nodes: project.nodes.map((n) => ({ id: n.id, descriptorId: n.descriptorId, params: n.params, position: [n.position.x, n.position.y] })),
  edges: project.edges.map((e) => ({ from: { node: e.source, port: e.sourceHandle }, to: { node: e.target, port: e.targetHandle } })),
};
assert.ok(!graph.nodes.some((n) => JSON.stringify(n.params).includes(oracle.answer)), 'Answer must not be a graph constant');
for (const secret of [oracle.keyHex, oracle.ivHex].filter(Boolean))
  assert.ok(!graph.nodes.some((n) => JSON.stringify(n.params).includes(secret)), 'Key and IV must come from the original fixture');
const child = spawn(build.executable, ['live_agent_bridge', '--ignored', '--nocapture', '--test-threads=1'], {
  env: { ...process.env, MISCLAB_LIVE_AI: '1' }, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'],
});
const failures = [], completed = new Set();
let settled = false;
const timer = setTimeout(() => { child.kill(); throw new Error('Replay timed out'); }, 30000);
child.on('error', (error) => { clearTimeout(timer); throw error; });
child.on('exit', (code) => { clearTimeout(timer); if (!settled) throw new Error(`Replay exited before returning results (${code})`); });
const lines = createInterface({ input: child.stdout });
lines.on('line', (line) => {
  // Rust's test runner can prefix the first message with the test name.
  const start = line.indexOf('AI_LIVE:');
  if (start < 0) return;
  const response = JSON.parse(line.slice(start + 8));
  if (response.id === 'ready') { child.stdin.write(JSON.stringify({ id: 'replay', op: 'run', graph }) + '\n'); return; }
  if (response.id !== 'replay') return;
  if (response.kind === 'event') {
    if (response.value.kind === 'nodeFailed') failures.push(response.value);
    if (response.value.kind === 'nodeDone') completed.add(response.value.node);
    return;
  }
  settled = true; clearTimeout(timer); child.stdin.end();
  assert.equal(response.kind, 'result', String(response.value));
  assert.deepEqual(failures, []);
  assert.equal(completed.size, graph.nodes.length);
  const finalNode = project.nodes.findLast((n) => n.descriptorId === 'text_output');
  const actual = response.value[finalNode.id].value.value;
  assert.equal(actual, oracle.answer);
  const evidence = { date: new Date().toISOString(), nodes: graph.nodes.length, edges: graph.edges.length,
    completed: completed.size, failures, finalNode: finalNode.id, actual, expectedMatch: true, modelCalls: 0, answerInParameters: false };
  fs.writeFileSync(evidencePath, JSON.stringify(evidence, null, 2));
  console.log(JSON.stringify(evidence));
});
