import {Platform} from "../../Platform";

const hash = require('@dashevo/dpp/lib/util/hash');
const crypto = require('crypto');

/**
 * Register names to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} name - name
 * @param {Object} records - records object having only one of the following items
 * @param {string} [records.dashUniqueIdentityId]
 * @param {string} [records.dashAliasIdentityId]
 * @param identity - identity
 *
 * @returns registered domain document
 */
export async function register(this: Platform,
                               name: string,
                               records: {
                                   dashUniqueIdentityId?: string,
                                   dashAliasIdentityId?: string,
                               },
                               identity: {
                                   getId(): string;
                                   getPublicKeyById(number: number):any;
                               },
): Promise<any> {
    const nameLabels = name.split('.');

    const normalizedParentDomainName = nameLabels
        .slice(1)
        .join('.')
        .toLowerCase();

    const [label] = nameLabels;
    const normalizedLabel = label.toLowerCase();

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
            label,
            normalizedLabel,
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
