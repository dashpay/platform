import { Platform } from "../../Platform";
import broadcastStateTransition from "../../broadcastStateTransition";
import { signStateTransition } from "../../signStateTransition";

/**
 * Broadcast contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param dataContract - contract
 * @param identity - identity
 * @return dataContract
 */
export default async function broadcast(this: Platform, dataContract: any, identity: any): Promise<any> {
    const { dpp } = this;

    const dataContractCreateTransition = dpp.dataContract.createStateTransition(dataContract);

    await signStateTransition(this, dataContractCreateTransition, identity);
    await broadcastStateTransition(this, dataContractCreateTransition);

    return dataContractCreateTransition;
}
