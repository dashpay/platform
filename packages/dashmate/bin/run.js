import { run } from '../dist/index.js';

await run();

// #!/usr/bin/env node
// console.log('start', new Date())
// async function main() {
//   console.log('main()', new Date())
//   const { execute } = await import('@oclif/core');
//   console.log('import()', new Date())
//   await execute({ dir: import.meta.url });
//   console.log('execute()', new Date())
// }
//
// await main();
