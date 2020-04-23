import {Platform} from "./Platform";

/**
 * @param {Platform} platform
 * @param stateTransition
 * @param identity
 */
export default async function broadcastStateTransition(platform: Platform, stateTransition: any, identity: any) {
    const { dpp, client, account } = platform;

    // @ts-ignore
    const { privateKey } = account.getIdentityHDKey(0);

    stateTransition.sign(
        identity.getPublicKeyById(0),
        privateKey,
    );

    // TODO: There is some bug internally for some reason
    // const result = await dpp.stateTransition.validateStructure(stateTransition);


    // if (!result.isValid()) {
    //     throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    // }

    await client.applyStateTransition(stateTransition);
}
