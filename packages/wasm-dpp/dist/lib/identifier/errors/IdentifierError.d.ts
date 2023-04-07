import { DPPError } from "../../errors/DPPError";
declare class IdentifierError extends DPPError {
    constructor(message: string);
}
export default IdentifierError;
