import {Platform} from "../../Platform";

import broadcastStateTransition from '../../broadcastStateTransition';

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
    const { dpp } = this;

    const records = {
        dashIdentity: identity.getId(),
    };

    const nameSlice = name.indexOf('.');
    const normalizedParentDomainName = (
        nameSlice === -1
        ? 'dash'
        : name.slice(nameSlice + 1)
    );
    const label = (
        nameSlice === -1
        ? name
        : name.slice(0,nameSlice)
    );
    const normalizedLabel = label.toLowerCase();
    const fullDomainName = `${normalizedLabel}.${normalizedParentDomainName}`;

    const nameHash = hash(
        Buffer.from(fullDomainName),
    ).toString('hex');

    const preorderSalt = entropy.generate();

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

    const preorderTransition = dpp.document.createStateTransition({
        create: [ preorderDocument ],
    });

    await broadcastStateTransition(this, preorderTransition, identity);

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
    const domainTransition = dpp.document.createStateTransition({
        create: [domainDocument],
    });

    await broadcastStateTransition(this, domainTransition, identity);

    return domainDocument;
}

export default register;
