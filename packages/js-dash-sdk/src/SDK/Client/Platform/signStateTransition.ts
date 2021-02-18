import {Platform} from "./Platform";

/**
 *
 * @param {Platform} platform
 * @param {AbstractStateTransition} stateTransition
 * @param {Identity} identity
 * @param {number} [keyIndex]
 * @return {AbstractStateTransition}
 */
export async function signStateTransition(platform: Platform, stateTransition: any, identity: any, keyIndex: number = 0): Promise<any> {
    const { client } = platform;

    const account = await client.getWalletAccount();

    // @ts-ignore
    const { privateKey } = account.identities.getIdentityHDKeyById(
        identity.getId().toString(),
        keyIndex,
    );

    stateTransition.sign(
        identity.getPublicKeyById(keyIndex),
        privateKey,
    );

    return stateTransition;
}