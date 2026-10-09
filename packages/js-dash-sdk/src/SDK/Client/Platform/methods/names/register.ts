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

  if (!this.client.getApps().has('dpns')) {
    throw new Error('DPNS is required to register a new name.');
  }

  // The preorder commits to the hash the platform checks the domain's
  // preorderSalt against. From DPNS v3 the contract declares it: the params of
  // the salt's findBy function (the writer's id, the salt, the normalized label,
  // '.' and the parent domain name as sent). Before it, the salt and
  // `${normalizedLabel}.${parentDomainName}`. The contract is the one the
  // network stores now, not one cached before an upgrade changed it, whose hash
  // the domain could not reveal; fetching it also replaces the cached one, so
  // both documents are created against it.
  const { contractId } = this.client.getApps().get('dpns');
  const dpnsContract = await this.contracts.get(contractId, { skipCache: true });
  const domainSchema = dpnsContract ? dpnsContract.getDocumentSchema('domain') : undefined;
  const hashParams = domainSchema?.properties?.preorderSalt?.refersTo
    ?.findBy?.saltedDomainHash?.params;

  const paramBytes = (param: any): Buffer => {
    if (typeof param === 'object' && param !== null) {
      return Buffer.from(param.const);
    }
    switch (param) {
      case '$ownerId':
        return Buffer.from(identity.getId().toBuffer());
      case 'preorderSalt':
        return preorderSalt;
      case 'label':
        return Buffer.from(label);
      case 'normalizedLabel':
        return Buffer.from(normalizedLabel);
      case 'parentDomainName':
        return Buffer.from(parentDomainName);
      case 'normalizedParentDomainName':
        return Buffer.from(normalizedParentDomainName);
      default:
        throw new Error(`Unknown DPNS preorder hash param ${param}`);
    }
  };

  const saltedDomainHash = hash(
    Array.isArray(hashParams)
      ? Buffer.concat(hashParams.map(paramBytes))
      : Buffer.concat([
        preorderSalt,
        Buffer.from(`${normalizedLabel}.${parentDomainName}`),
      ]),
  );

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
