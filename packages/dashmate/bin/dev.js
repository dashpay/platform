#!/usr/bin/env node

import { execute } from '@oclif/core';

async function run() {
  await execute({ dir: import.meta.url, development: true });
}

await run();
