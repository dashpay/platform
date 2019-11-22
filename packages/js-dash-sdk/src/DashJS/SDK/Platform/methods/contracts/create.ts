import {Platform} from "../../Platform";

export function create(this: Platform): any {
    throw new Error('Implementation missing in dependencies.');
}

export default create;
