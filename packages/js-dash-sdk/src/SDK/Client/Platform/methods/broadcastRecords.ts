import {Platform} from "../../Platform";
import StateTransitionBuilder, {StateTransitionBuilderTypes} from "../../StateTransitionBuilder/StateTransitionBuilder";

/**
 * Register the set of records provided by creating and broadcasting a stateTransition.
 *
 * @param {Platform} this - bound instance class
 * @param records - records to register onto platform
 * @param identity - identity
 */
export async function broadcastRecords(this: Platform, records: [any], identity: any): Promise<any> {
    const {account, client, dpp} = this;

    const builder = new StateTransitionBuilder({
        dpp,
        client
    });
    builder.addRecord(records);

    const identityType = (identity.getType() === 2) ? 'application' : 'user';

    if(builder.type === StateTransitionBuilderTypes.CONTRACT && identityType !== 'application'){
        throw new Error('An Application Identity is required to broadcast.');
    }
    // @ts-ignore
    const identityPrivateKey = account.getIdentityHDKey(0, identityType).privateKey;

    await builder.register(identity, identityPrivateKey);
}

export default broadcastRecords;
