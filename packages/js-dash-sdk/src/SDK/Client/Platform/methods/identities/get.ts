import {Platform} from "../../Platform";

/**
 * Get an identity from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} id - id
 * @returns Identity
 */
export async function get(this: Platform, id: string): Promise<any> {
    // @ts-ignore
    const identityBuffer = await this.client.getDAPIClient().platform.getIdentity(id);

    if (identityBuffer === null) {
        return null;
    }

    return this.dpp.identity.createFromSerialized(identityBuffer);
}

export default get;
