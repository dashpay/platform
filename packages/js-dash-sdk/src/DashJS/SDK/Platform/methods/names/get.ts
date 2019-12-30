import {Platform} from "../../Platform";

export async function get(this: Platform, id: string): Promise<any> {
    throw new Error('Implementation missing in dependencies.');
}

export default get;
