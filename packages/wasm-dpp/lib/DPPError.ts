export class DPPError extends Error {
    name: string;
    message: string;

    constructor(message: string) {
        super();

        this.name = this.constructor.name;
        this.message = message;

        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, this.constructor);
        }
    }
}
