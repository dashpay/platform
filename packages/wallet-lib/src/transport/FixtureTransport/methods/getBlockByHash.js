import dashcore from '@dashevo/dashcore-lib';
const { Block } = dashcore;
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import blocks from '../data/blocks/blocks.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default async function getBlockByHash(hash) {
  const height = blocks.hashes[hash];
  const blockfile = JSON.parse(fs.readFileSync(`${__dirname}/../data/blocks/${height}.json`));
  return new Block(Buffer.from(blockfile.block, 'hex'));
}
