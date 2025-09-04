/**
 * Known Data Contract Schemas for Document Explorer
 * Provides contract definitions and field information for query building
 */

export const KNOWN_CONTRACTS = {
    dpns: {
        name: 'DPNS (Dash Platform Name Service)',
        id: {
            testnet: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            mainnet: '566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy'
        },
        description: 'Manages usernames and domain resolution on Dash Platform',
        documents: {
            domain: {
                description: 'Domain registration records',
                fields: {
                    '$ownerId': { type: 'identifier', description: 'Identity that owns this domain' },
                    'label': { type: 'string', description: 'Domain label (e.g., "alice" for alice.dash)' },
                    'normalizedLabel': { type: 'string', description: 'Normalized domain label' },
                    'normalizedParentDomainName': { type: 'string', description: 'Parent domain name' },
                    'preorderSalt': { type: 'array', description: 'Salt used in preorder process' },
                    'records': { type: 'object', description: 'DNS-like records' },
                    'subdomainRules': { type: 'object', description: 'Subdomain creation rules' },
                    '$createdAt': { type: 'date', description: 'Creation timestamp' },
                    '$updatedAt': { type: 'date', description: 'Last update timestamp' }
                }
            },
            preorder: {
                description: 'Domain preorder records for name registration',
                fields: {
                    '$ownerId': { type: 'identifier', description: 'Identity preordering the domain' },
                    'saltedDomainHash': { type: 'array', description: 'Salted hash of domain name' },
                    '$createdAt': { type: 'date', description: 'Creation timestamp' }
                }
            }
        }
    },

    dashpay: {
        name: 'DashPay (Social Payments)',
        id: {
            testnet: 'FQco85WbwNgb5ix8DFFX7oGYzj5bkgs4AFdrZx4gBMzV',
            mainnet: 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7'
        },
        description: 'Enables social payment features and contact management',
        documents: {
            profile: {
                description: 'User profiles with display information',
                fields: {
                    '$ownerId': { type: 'identifier', description: 'Identity that owns this profile' },
                    'displayName': { type: 'string', description: 'User display name' },
                    'publicMessage': { type: 'string', description: 'Public message/bio' },
                    'avatarUrl': { type: 'string', description: 'Avatar image URL' },
                    'avatarHash': { type: 'array', description: 'Avatar image hash' },
                    'avatarFingerprint': { type: 'array', description: 'Avatar fingerprint' },
                    '$createdAt': { type: 'date', description: 'Creation timestamp' },
                    '$updatedAt': { type: 'date', description: 'Last update timestamp' }
                }
            },
            contactInfo: {
                description: 'Contact information and preferences',
                fields: {
                    '$ownerId': { type: 'identifier', description: 'Identity that owns this contact info' },
                    'encryptedPubKey': { type: 'array', description: 'Encrypted public key' },
                    'rootEncryptionKeyIndex': { type: 'integer', description: 'Root encryption key index' },
                    'derivationEncryptionKeyIndex': { type: 'integer', description: 'Derivation key index' },
                    '$createdAt': { type: 'date', description: 'Creation timestamp' }
                }
            },
            contactRequest: {
                description: 'Contact requests between users',
                fields: {
                    '$ownerId': { type: 'identifier', description: 'Requesting identity' },
                    'toUserId': { type: 'identifier', description: 'Target user identity' },
                    'encryptedPubKey': { type: 'array', description: 'Encrypted public key' },
                    'senderKeyIndex': { type: 'integer', description: 'Sender key index' },
                    'recipientKeyIndex': { type: 'integer', description: 'Recipient key index' },
                    'accountReference': { type: 'integer', description: 'Account reference' },
                    '$createdAt': { type: 'date', description: 'Creation timestamp' }
                }
            }
        }
    },

    withdrawals: {
        name: 'Withdrawals',
        id: {
            testnet: 'FH1kEzC6FZEyA8Jb3V4zzJq6a2Y8XuGY7KLJ5RzEzQxY',
            mainnet: 'DMP8SE35TuJvWZEK5DNSjh8gLhNE6YXnVMPQeZjqZxPm'
        },
        description: 'Manages platform credit withdrawals to Layer 1',
        documents: {
            withdrawal: {
                description: 'Withdrawal transaction records',
                fields: {
                    '$ownerId': { type: 'identifier', description: 'Identity performing withdrawal' },
                    'amount': { type: 'integer', description: 'Amount to withdraw in credits' },
                    'coreFeePerByte': { type: 'integer', description: 'Core fee per byte' },
                    'pooling': { type: 'integer', description: 'Pooling method' },
                    'outputScript': { type: 'array', description: 'Output script for withdrawal' },
                    'status': { type: 'integer', description: 'Withdrawal status' },
                    '$createdAt': { type: 'date', description: 'Creation timestamp' },
                    '$updatedAt': { type: 'date', description: 'Last update timestamp' }
                }
            }
        }
    },

    // Generic contract template for custom contracts
    custom: {
        name: 'Custom Contract',
        id: {
            testnet: '',
            mainnet: ''
        },
        description: 'User-defined data contract',
        documents: {
            // Will be populated dynamically when contract is loaded
        }
    }
};

/**
 * Get contract schema by name and network
 */
export function getContractSchema(contractName, network = 'testnet') {
    const contract = KNOWN_CONTRACTS[contractName];
    if (!contract) return null;

    return {
        ...contract,
        contractId: contract.id[network]
    };
}

/**
 * Get all available fields for a document type
 */
export function getDocumentFields(contractName, documentType) {
    const contract = KNOWN_CONTRACTS[contractName];
    if (!contract || !contract.documents[documentType]) {
        return [];
    }

    const fields = contract.documents[documentType].fields;
    return Object.keys(fields).map(fieldName => ({
        name: fieldName,
        type: fields[fieldName].type,
        description: fields[fieldName].description
    }));
}

/**
 * Get common query fields for a document type (fields that make sense for WHERE/ORDER BY)
 */
export function getQueryableFields(contractName, documentType) {
    const allFields = getDocumentFields(contractName, documentType);
    
    // Filter to fields that are commonly queryable
    const queryableTypes = ['string', 'integer', 'date', 'identifier', 'boolean'];
    
    return allFields.filter(field => {
        return queryableTypes.includes(field.type) || field.name.startsWith('$');
    });
}

/**
 * Get sample queries for demonstration
 */
export function getSampleQueries(contractName, documentType, network = 'testnet') {
    const samples = {
        dpns: {
            domain: [
                {
                    name: "Recent domains",
                    where: [],
                    orderBy: [['$createdAt', 'desc']],
                    limit: 10
                },
                {
                    name: "Domains by owner",
                    where: [['$ownerId', '=', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk']],
                    orderBy: [['$createdAt', 'desc']],
                    limit: 20
                },
                {
                    name: "Short labels",
                    where: [['label', 'length', '<', 6]],
                    orderBy: [['label', 'asc']],
                    limit: 15
                }
            ],
            preorder: [
                {
                    name: "Recent preorders",
                    where: [],
                    orderBy: [['$createdAt', 'desc']],
                    limit: 10
                }
            ]
        },
        dashpay: {
            profile: [
                {
                    name: "All profiles",
                    where: [],
                    orderBy: [['$createdAt', 'desc']],
                    limit: 20
                },
                {
                    name: "Profiles with avatars",
                    where: [['avatarUrl', '!=', null]],
                    orderBy: [['$updatedAt', 'desc']],
                    limit: 10
                }
            ],
            contactRequest: [
                {
                    name: "Recent requests",
                    where: [],
                    orderBy: [['$createdAt', 'desc']],
                    limit: 10
                }
            ]
        },
        withdrawals: {
            withdrawal: [
                {
                    name: "Recent withdrawals",
                    where: [],
                    orderBy: [['$createdAt', 'desc']],
                    limit: 10
                },
                {
                    name: "Large withdrawals",
                    where: [['amount', '>', 1000000000]], // > 10 DASH
                    orderBy: [['amount', 'desc']],
                    limit: 5
                }
            ]
        }
    };

    return samples[contractName]?.[documentType] || [];
}

/**
 * Validate where clause operators for field type
 */
export function getValidOperators(fieldType) {
    const operatorsByType = {
        string: ['=', '!=', 'startsWith', 'in'],
        integer: ['=', '!=', '>', '>=', '<', '<=', 'in'],
        date: ['=', '!=', '>', '>=', '<', '<='],
        identifier: ['=', '!=', 'in'],
        boolean: ['=', '!='],
        array: ['=', '!=', 'in'],
        object: ['=', '!=']
    };

    return operatorsByType[fieldType] || ['=', '!='];
}

/**
 * Format field value for display
 */
export function formatFieldValue(value, fieldType) {
    switch (fieldType) {
        case 'date':
            return new Date(value).toLocaleString();
        case 'identifier':
            return value.length > 20 ? `${value.substring(0, 8)}...${value.substring(-8)}` : value;
        case 'array':
            return Array.isArray(value) ? `[${value.length} items]` : value;
        case 'object':
            return typeof value === 'object' ? JSON.stringify(value, null, 2) : value;
        default:
            return value;
    }
}