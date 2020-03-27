// @ts-ignore
import DAPIClient from "@dashevo/dapi-client";

import {PrivateKey} from "@dashevo/dashcore-lib";
// @ts-ignore
import Document from "@dashevo/dpp/lib/document/Document";
// @ts-ignore
import DataContract from "@dashevo/dpp/lib/dataContract/DataContract";
// @ts-ignore
import Identity from "@dashevo/dpp/lib/identity/Identity";
// @ts-ignore
import DashPlatformProtocol from "@dashevo/dpp";
import getTypeOfRecord from "./getTypeOfRecord";
import {ClientDependency} from "../Client";

export const enum StateTransitionBuilderTypes {
    CONTRACT = 'dataContract',
    DOCUMENT = 'document',
    IDENTITY = 'identity',
}

export type Record = Document | DataContract | Identity;

export interface StateTransitionBuilderOpts {
    dpp: DashPlatformProtocol,
    client?: ClientDependency
};

/**
 * Builder for ST. Allows to manage and broadcast a set of record
 *
 * @param {StateTransitionBuilderTypes} type - a valid st builder type
 * @param dpp - DashPlatformProtocol instance
 * @param client - ClientDependency instance
 */
export class StateTransitionBuilder {
    public records: Record[];
    public type: null | StateTransitionBuilderTypes;

    private dpp: DashPlatformProtocol | undefined;
    private client: ClientDependency | undefined;

    constructor(opts: StateTransitionBuilderOpts) {
        this.type = null;
        if (opts.client) this.client = opts.client;
        if (opts.dpp === undefined) {
            throw new Error('Records requires a DPP instance for stateTransition creation');
        }
        this.dpp = opts.dpp;
        this.records = [];
    }

    /**
     * Allow to add a new record
     * @param record - a valid record
     */
    addRecord(record: Record) {
        if (Array.isArray(record)) {
            record.forEach((_record) => this.addRecord(_record))
            return;
        }
        let recordType = getTypeOfRecord(record);
        if (!this.type) this.type = recordType;
        if (recordType !== this.type) {
            throw new Error(`Records cannot add to records of type ${this.type}: record type ${recordType}`);
        }
        this.records.push(record);
    }

    /**
     * Register the records to the platform by broadcasting a state transition.
     *
     * @param {Identity} identity - identity with which broadcast theses records
     * @param {PrivateKey} identityPrivateKey - private key associate to the identity for ST signing.
     */
    async register(identity: Identity, identityPrivateKey: PrivateKey) {
        const dapiClient = this.client;
        if (!dapiClient) {
            throw new Error('Requires a DAPIClient instance for stateTransition creation');
        }
        let stateTransition = await this.toStateTransition();
        //
        //ECDSA_SECP256K1
        const identityPublicKey = identity.getPublicKeyById(1);
        // @ts-ignore
        stateTransition.sign(identityPublicKey, identityPrivateKey);

        const result = await this.dpp.stateTransition.validateStructure(stateTransition);
        if(!result.isValid()){
            throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
        }

        // @ts-ignore
        await dapiClient.applyStateTransition(stateTransition);

    }

    /**
     * Returns a StateTransition containing the records
     * @return {DataContractStateTransition|DocumentsStateTransition|IdentityCreateTransition}
     */
    async toStateTransition() {
        if (!this.type || !this.records.length) throw new Error('Need record to create a StateTransition');
        const records = (this.type === StateTransitionBuilderTypes.CONTRACT) ? this.records[0] : this.records;
        return this.dpp[this.type].createStateTransition(records);
    }
}

export default StateTransitionBuilder;
