import {Platform} from "../../Platform";
// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";

/**
 * Get an identity from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string|Identifier} id - id
 * @returns Identity
 */
export async function get(this: Platform, id: Identifier|string): Promise<any> {
    await this.initialize();

    const identifier = Identifier.from(id);

    // @ts-ignore
    const identityBuffer = await this.client.getDAPIClient().platform.getIdentity(identifier);

    if (identityBuffer === null) {
        return null;
    }

    return this.dpp.identity.createFromBuffer(identityBuffer);
}

export default get;
