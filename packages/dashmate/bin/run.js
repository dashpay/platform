async function main() {
  const { execute } = await import('@oclif/core');
  await execute({ dir: '/Users/ivanshumkov/Projects/dashpay/platform/packages/dashmate/', development: true });
}

await main();
