import {Platform} from "../../Platform";

const entropy = require('@dashevo/dpp/lib/util/entropy');
const { hash } = require('@dashevo/dpp/lib/util/multihashDoubleSHA256');
const bs58 = require('bs58');

/**
 * Register names to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} name - name
 * @param identity - identity
 * @param {any} [identity.id] - identity ID
 * @param {function(number):any} - get public key by ID
 * @returns registered names
 */
export async function register(this: Platform,
                               name: string,
                               identity: {
                                   getId(): string;
                                   getPublicKeyById(number: number):any;
                               }
): Promise<any> {
    const records = {
        dashIdentity: identity.getId(),
    };

    const nameLabels = name.split('.');

    const normalizedParentDomainName = nameLabels
        .slice(1)
        .join('.')
        .toLowerCase();

    const [label] = nameLabels;
    const normalizedLabel = label.toLowerCase();

    const preorderSalt = entropy.generate();

    const fullDomainName = normalizedParentDomainName.length > 0
        ? `${normalizedLabel}.${normalizedParentDomainName}`
        : normalizedLabel;

    const nameHash = hash(
        Buffer.from(fullDomainName),
    ).toString('hex');

    const saltedDomainHashBuffer = Buffer.concat([
        bs58.decode(preorderSalt),
        Buffer.from(nameHash, 'hex'),
    ]);

    const saltedDomainHash = hash(
        saltedDomainHashBuffer,
    ).toString('hex');

    if (!this.apps.dpns.contractId) {
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
            nameHash,
            label,
            normalizedLabel,
            normalizedParentDomainName,
            preorderSalt,
            records,
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
