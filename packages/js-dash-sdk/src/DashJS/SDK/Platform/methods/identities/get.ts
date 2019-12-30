import {Platform} from "../../Platform";

export async function get(this: Platform, id: string): Promise<any> {
    // @ts-ignore
    const identityBuffer = await this.client.getIdentity(id);
    if(identityBuffer===null){
        return null;
    }
    return this.dpp.identity.createFromSerialized(identityBuffer);
}

export default get;
