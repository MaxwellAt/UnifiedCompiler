const os = require('os');

console.log("=".repeat(60));
console.log(`  [NODE.JS CLUSTER JOB] Running on Remote Host: ${os.hostname()}`);
console.log(`  Node.js Version: ${process.version}`);
console.log(`  Architecture   : ${process.arch} | Platform: ${process.platform}`);
console.log(`  CPUs Available : ${os.cpus().length} cores (${os.cpus()[0].model})`);
console.log(`  Free Memory    : ${(os.freemem() / 1024 / 1024).toFixed(0)} MB / ${(os.totalmem() / 1024 / 1024).toFixed(0)} MB`);
console.log("=".repeat(60));

console.log("Executing asynchronous async-pipeline simulation...");
const startTime = Date.now();

function simulateTask(id, delayMs) {
  return new Promise((resolve) => {
    setTimeout(() => {
      console.log(`  [Worker ${id}] Task chunk processed in ${delayMs}ms`);
      resolve(id * 10);
    }, delayMs);
  });
}

async function main() {
  const tasks = [
    simulateTask(1, 100),
    simulateTask(2, 200),
    simulateTask(3, 150),
    simulateTask(4, 80),
  ];

  const results = await Promise.all(tasks);
  const total = results.reduce((acc, curr) => acc + curr, 0);

  console.log("-".repeat(60));
  console.log(`  All remote workers completed! Aggregated sum: ${total}`);
  console.log(`  Total execution time: ${Date.now() - startTime}ms`);
  console.log("=".repeat(60));
}

main();
