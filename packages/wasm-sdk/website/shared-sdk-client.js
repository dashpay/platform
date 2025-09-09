// Client for SharedWorker SDK
export class SharedSdkClient {
    constructor() {
        this.worker = null;
        this.port = null;
        this.requests = new Map();
        this.requestId = 0;
        this.onProgress = null;
        this.isConnected = false;
    }
    
    async connect() {
        if (this.isConnected) return;
        
        return new Promise((resolve, reject) => {
            try {
                // Create or connect to shared worker
                this.worker = new SharedWorker('./shared-sdk-worker.js', {
                    type: 'module',
                    name: 'dash-wasm-sdk'
                });
                
                this.port = this.worker.port;
                
                // Handle messages from worker
                this.port.onmessage = (e) => {
                    const { type, id, result, error, percent, text, initialized } = e.data;
                    
                    switch (type) {
                        case 'connected':
                            this.isConnected = true;
                            resolve(initialized);
                            break;
                            
                        case 'progress':
                            if (this.onProgress) {
                                this.onProgress(percent, text);
                            }
                            break;
                            
                        case 'initialized':
                            // SDK initialization complete
                            break;
                            
                        case 'result':
                            const request = this.requests.get(id);
                            if (request) {
                                request.resolve(result);
                                this.requests.delete(id);
                            }
                            break;
                            
                        case 'error':
                            const errRequest = this.requests.get(id);
                            if (errRequest) {
                                errRequest.reject(new Error(error));
                                this.requests.delete(id);
                            }
                            break;
                            
                        case 'status':
                            const statusRequest = this.requests.get(id);
                            if (statusRequest) {
                                statusRequest.resolve(initialized);
                                this.requests.delete(id);
                            }
                            break;
                    }
                };
                
                this.port.start();
                
            } catch (error) {
                reject(error);
            }
        });
    }
    
    async initialize() {
        if (!this.isConnected) {
            await this.connect();
        }
        
        return this.sendRequest('init');
    }
    
    async checkStatus() {
        if (!this.isConnected) {
            await this.connect();
        }
        
        return this.sendRequest('checkStatus');
    }
    
    async execute(method, ...args) {
        if (!this.isConnected) {
            await this.connect();
        }
        
        return this.sendRequest('execute', { method, args });
    }
    
    sendRequest(type, data = {}) {
        return new Promise((resolve, reject) => {
            const id = this.requestId++;
            this.requests.set(id, { resolve, reject });
            
            this.port.postMessage({
                type,
                id,
                ...data
            });
        });
    }
    
    // Proxy methods to match SDK interface
    get_identity(...args) {
        return this.execute('get_identity', ...args);
    }
    
    get_identity_keys(...args) {
        return this.execute('get_identity_keys', ...args);
    }
    
    get_identity_balance(...args) {
        return this.execute('get_identity_balance', ...args);
    }
    
    get_data_contract(...args) {
        return this.execute('get_data_contract', ...args);
    }
    
    get_documents(...args) {
        return this.execute('get_documents', ...args);
    }
    
    // Add more proxy methods as needed...
}

// Create a singleton instance
export const sharedSdk = new SharedSdkClient();