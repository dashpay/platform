import dashcore from '@dashevo/dashcore-lib';
const { Transaction } = dashcore;
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default async function getTransaction(transactionHash) {
  const txFile = JSON.parse(fs.readFileSync(`${__dirname}/../data/transactions/${transactionHash}.json`));
  return new Transaction(Buffer.from(txFile.transaction, 'hex'));
}
