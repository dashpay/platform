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

  const isSecondLevelDomain = normalizedParentDomainName.length > 0;

  if (!this.client.getApps().has('dpns')) {
    throw new Error('DPNS is required to register a new name.');
  }

  // The preorder holds the hashes the platform finds it by. From DPNS v3 the
  // contract declares them: the domain's creatorRefersTo finds the creator's own
  // preorder by a findBy function over the creator's id, the normalized label,
  // '.' and the normalized parent, which the preorder holds under the property
  // the function fills, and the same params without the creator's id give its
  // nameHash. Before it, the domain reveals a salt: the preorder holds
  // saltedDomainHash, of the salt and `${normalizedLabel}.${parentDomainName}`,
  // or the label as sent for a top-level name. The contract is the one the
  // network stores now, not one cached before an upgrade changed it, whose hash
  // the domain could not reveal; fetching it also replaces the cached one, so
  // both documents are created against it.
  const { contractId } = this.client.getApps().get('dpns');
  const dpnsContract = await this.contracts.get(contractId, { skipCache: true });
  const domainSchema = dpnsContract ? dpnsContract.getDocumentSchema('domain') : undefined;
  const [hashProperty, hashFunction] = Object.entries(
    domainSchema?.creatorRefersTo?.findBy ?? {},
  ).find(([, source]: [string, any]) => Array.isArray(source?.params)) ?? [];
  const hashParams: any[] | undefined = (hashFunction as any)?.params;

  // From DPNS v3 a new name's records.identity must be its owner
  // (recordsIdentityIsOwner): refuse another before the preorder is paid
  if (domainSchema?.propertyConstraints?.recordsIdentityIsOwner
    && records.identity
    && !Identifier.from(records.identity).equals(identity.getId())) {
    throw new Error('records.identity must be the identity registering the name.');
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
      nameHash: hash(Buffer.concat(
        hashParams.filter((param) => param !== '$ownerId').map(paramBytes),
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
