import { Identifier } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import convertToHomographSafeChars from '../../../../../utils/convertToHomographSafeChars';

const crypto = require('crypto');
const { hash } = require('@dashevo/wasm-dpp/lib/utils/hash');

// How long before a pending preorder expires a registration stops revealing it: a domain
// create broadcast now lands in a later block, and a preorder expired at that block's time
// is not found. One minute covers the blocks a broadcast waits for.
const PREORDER_REVEAL_MARGIN_MS = 60_000;

/**
 * Whether a domain create broadcast now can still reveal `preorder`, of a type whose
 * documents live `ttlSeconds`: the platform counts it expired from its $createdAt plus
 * the time to live, at the time of the block the reveal lands in. Now is the time of the
 * block the query that found it reports, or the local clock when it reports none. A
 * preorder of a type with no time to live, or with no $createdAt, never expires.
 */
function isRevealable(preorder: any, ttlSeconds: number | undefined): boolean {
  const createdAt = preorder.getCreatedAt?.();
  if (typeof ttlSeconds !== 'number' || !createdAt) {
    return true;
  }
  const reportedTimeMs = preorder.getMetadata?.()?.getTimeMs();
  const nowMs = reportedTimeMs !== undefined ? Number(reportedTimeMs) : Date.now();
  return nowMs + PREORDER_REVEAL_MARGIN_MS < createdAt.getTime() + ttlSeconds * 1000;
}

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

  const isSecondLevelDomain = normalizedParentDomainName.length > 0;

  if (!this.client.getApps().has('dpns')) {
    throw new Error('DPNS is required to register a new name.');
  }

  // The preorder holds the hashes the platform finds and judges it by. From DPNS
  // v3 the contract declares them: the domain's creatorRefersTo finds the
  // creator's own preorder by a findBy function over the normalized label, '.'
  // and the normalized parent, which the preorder holds under the property the
  // function fills (nameHash), and its chooseFirst counts a preorder holding,
  // under its validWhen property (ownerNameHash), the hash of its owner's id and
  // the same values. Before it, the domain reveals a salt: the preorder holds
  // saltedDomainHash, of the salt and `${normalizedLabel}.${parentDomainName}`,
  // or the label as sent for a top-level name. The contract is the one the
  // network stores now, not one cached before an upgrade changed it, whose hash
  // the domain could not reveal; fetching it also replaces the cached one, so
  // both documents are created against it.
  const { contractId } = this.client.getApps().get('dpns');
  const dpnsContract = await this.contracts.get(contractId, { skipCache: true });
  const domainSchema = dpnsContract ? dpnsContract.getDocumentSchema('domain') : undefined;
  const creatorRefersTo = domainSchema?.creatorRefersTo;
  const functionOf = (entries: Record<string, any> | undefined) => Object.entries(
    entries ?? {},
  ).find(([, source]: [string, any]) => Array.isArray(source?.params)) ?? [];
  const [hashProperty, hashFunction] = functionOf(creatorRefersTo?.findBy);
  const hashParams: any[] | undefined = hashFunction?.params;
  const [validWhenProperty, validWhenFunction] = functionOf(
    creatorRefersTo?.constraints?.chooseFirst?.validWhen,
  );
  const validWhenParams: any[] | undefined = validWhenFunction?.params;
  if (Array.isArray(hashParams) && !Array.isArray(validWhenParams)) {
    throw new Error('The DPNS domain\'s creatorRefersTo declares no chooseFirst validWhen function.');
  }

  const paramBytes = (param: any): Buffer => {
    if (typeof param === 'object' && param !== null) {
      return Buffer.from(param.const);
    }
    switch (param) {
      case '$ownerId':
        return Buffer.from(identity.getId().toBuffer());
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

  const preorderSalt = crypto.randomBytes(32);
  const preorderProperties = Array.isArray(hashParams)
    ? {
      [hashProperty as string]: hash(Buffer.concat(hashParams.map(paramBytes))),
      [validWhenProperty as string]: hash(Buffer.concat(
        (validWhenParams as any[]).map(paramBytes),
      )),
    }
    : {
      saltedDomainHash: hash(Buffer.concat([
        preorderSalt,
        Buffer.from(isSecondLevelDomain ? `${normalizedLabel}.${parentDomainName}` : label),
      ])),
    };

  // 1. Create preorder document. From DPNS v3 an identity holds one pending
  // preorder of a name: one an earlier registration left behind, whose domain
  // create failed, is revealed again rather than preordered anew, which the
  // platform would refuse as a duplicate. A failed query is an error.
  const [pendingPreorder] = Array.isArray(hashParams)
    ? await this.documents.get('dpns.preorder', {
      where: [
        ['$ownerId', '==', identity.getId()],
        [hashProperty as string, '==', preorderProperties[hashProperty as string]],
      ],
      limit: 1,
    })
    : [];

  // One that expires before a reveal could land is not found by the reveal, and while
  // the cleanup of expired documents has not reached it, it still holds the owner's
  // entry of the name, which refuses a new preorder as a duplicate. This SDK reads
  // documents and contracts without proofs, so it signs nothing destructive on what a
  // node answered: it stops before paying, and the name can be preordered again once
  // the cleanup has removed the expired preorder.
  if (pendingPreorder
    && !isRevealable(pendingPreorder, dpnsContract?.getDocumentSchema('preorder')?.ttl)) {
    throw new Error('The pending preorder of this name expires before its reveal could land. '
      + 'Retry once it has expired and the platform has removed it.');
  }

  if (!pendingPreorder) {
    const preorderDocument = await this.documents.create(
      'dpns.preorder',
      identity,
      preorderProperties,
    );

    await this.documents.broadcast(
      {
        create: [preorderDocument],
      },
      identity,
    );
  }

  // 3. Create domain document
  const domainDocument = await this.documents.create(
    'dpns.domain',
    identity,
    {
      label,
      normalizedLabel,
      parentDomainName,
      normalizedParentDomainName,
      ...(Array.isArray(hashParams) ? {} : { preorderSalt }),
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
