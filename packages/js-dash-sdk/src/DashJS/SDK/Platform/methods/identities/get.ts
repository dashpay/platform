import {Platform} from "../../Platform";

export function get(this: Platform): any {
    throw new Error('Implementation missing in dependencies.');

}

export default get;
