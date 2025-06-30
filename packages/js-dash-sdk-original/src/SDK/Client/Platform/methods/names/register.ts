import { Identifier } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import convertToHomographSafeChars from '../../../../../utils/convertToHomographSafeChars';

const crypto = require('crypto');
const { hash } = require('@dashevo/wasm-dpp/lib/utils/hash');

/**
 * Register names to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} name - name
 * @param {Object} records - records object having only one of the following items
 * @param {string} [records.identity]
 * @param identity - identity
 *
 * @returns registered domain document
 */
export async function register(
  this: Platform,
  name: string,
  records: {
    identity?: Identifier | string,
  },
  identity: {
    getId(): Identifier;
    getPublicKeyById(number: number):any;
  },
): Promise<any> {
  await this.initialize();

  if (records.identity) {
    records.identity = Identifier.from(records.identity);
  }

  const nameLabels = name.split('.');

  const parentDomainName = nameLabels
    .slice(1)
    .join('.');

  const normalizedParentDomainName = convertToHomographSafeChars(parentDomainName);

  const [label] = nameLabels;
  const normalizedLabel = convertToHomographSafeChars(label);

  const preorderSalt = crypto.randomBytes(32);

  const isSecondLevelDomain = normalizedParentDomainName.length > 0;

  const fullDomainName = isSecondLevelDomain
    ? `${normalizedLabel}.${normalizedParentDomainName}`
    : normalizedLabel;

  const saltedDomainHash = hash(
    Buffer.concat([
      preorderSalt,
      Buffer.from(fullDomainName),
    ]),
  );

  if (!this.client.getApps().has('dpns')) {
    throw new Error('DPNS is required to register a new name.');
  }

  // 1. Create preorder document
  const preorderDocument = await this.documents.create(
    'dpns.preorder',
    identity,
    {
      saltedDomainHash,
    },
  );

  await this.documents.broadcast(
    {
      create: [preorderDocument],
    },
    identity,
  );

  // 3. Create domain document
  const domainDocument = await this.documents.create(
    'dpns.domain',
    identity,
    {
      label,
      normalizedLabel,
      parentDomainName,
      normalizedParentDomainName,
      preorderSalt,
      records,
      subdomainRules: {
        allowSubdomains: !isSecondLevelDomain,
      },
    },
  );

  // 4. Create and send domain state transition
  await this.documents.broadcast(
    {
      create: [domainDocument],
    },
    identity,
  );

  return domainDocument;
}

export default register;
