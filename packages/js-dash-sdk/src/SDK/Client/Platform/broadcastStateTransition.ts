import { Platform } from "./Platform";
import { wait } from "../../../utils/wait";

/**
 * @param {Platform} platform
 * @param stateTransition
 * @param identity
 * @param {number} [keyIndex=0]
 */
export default async function broadcastStateTransition(platform: Platform, stateTransition: any, identity: any, keyIndex : number = 0) {
    const { client, dpp } = platform;

    const account = await client.getWalletAccount();

    // @ts-ignore
    const { privateKey } = account.getIdentityHDKeyById(
        identity.getId(),
        keyIndex,
    );

    stateTransition.sign(
        identity.getPublicKeyById(keyIndex),
        privateKey,
    );

    const result = await dpp.stateTransition.validateStructure(stateTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    await client.getDAPIClient().platform.broadcastStateTransition(stateTransition.serialize());

    // Wait some time for propagation
    await wait(1000);
}
