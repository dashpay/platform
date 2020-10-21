import {Platform} from "../../Platform";
import Identifier from "@dashevo/dpp/lib/Identifier";

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
                                   dashUniqueIdentityId?: Identifier|string,
                                   dashAliasIdentityId?: Identifier|string,
                               },
                               identity: {
                                   getId(): Identifier;
                                   getPublicKeyById(number: number):any;
                               },
): Promise<any> {
    if (records.dashUniqueIdentityId) {
        records.dashUniqueIdentityId = Identifier.from(records.dashUniqueIdentityId);
    }

    if (records.dashAliasIdentityId) {
        records.dashAliasIdentityId = Identifier.from(records.dashAliasIdentityId);
    }

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
