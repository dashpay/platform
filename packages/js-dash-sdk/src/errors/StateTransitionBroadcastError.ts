export class StateTransitionBroadcastError extends Error {
    code: number;
    message: string;
    data: any;

    /**
     *
     * @param {number} code
     * @param {string} message
     * @param {*} data
     */
    constructor(code: number, message: string, data: any) {
        let detailedMessage = message;

        if (data && data.errors && data.errors.length > 0) {
            const [firstError] = data.errors;
            
            detailedMessage += `: ${firstError.name}: ${firstError.message}`;
            
            if (data.errors.length > 1) {
              detailedMessage += ` and ${data.errors.length} more`;
            }
        }

        super(detailedMessage);

        this.code = code;
        this.message = detailedMessage;
        this.data = data;

        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, this.constructor);
        }

        Object.setPrototypeOf(this, StateTransitionBroadcastError.prototype);
    }

    /**
     * Returns error code
     *
     * @return {number}
     */
    getCode(): number {
        return this.code;
    }

    /**
     * Returns error message
     *
     * @return {string}
     */
    getMessage(): string {
        return this.message;
    }

    /**
     * Get error data
     *
     * @return {*}
     */
    getData(): any {
        return this.data;
    }
}
