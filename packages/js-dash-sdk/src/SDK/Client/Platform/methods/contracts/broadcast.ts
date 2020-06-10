/**
 * Broadcast contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param dataContract - contract
 * @param identity - identity
 * @return dataContract
 */
import {Platform} from "../../Platform";
import broadcastStateTransition from "../../broadcastStateTransition";

export default async function broadcast(this: Platform, dataContract: any, identity: any): Promise<any> {
    const { dpp } = this;

    const dataContractCreateTransition = dpp.dataContract.createStateTransition(dataContract);

    await broadcastStateTransition(this, dataContractCreateTransition, identity);

    return dataContract;
}
