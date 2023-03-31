import {DPPError} from "../../errors/DPPError";

class IdentifierError extends DPPError {
  constructor(message: string) {
    super(message);
  }
}

export default IdentifierError;
