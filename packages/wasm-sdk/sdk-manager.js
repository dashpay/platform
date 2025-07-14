// SDK Manager - Simple approach using iframe
class SdkManager {
    constructor() {
        this.iframe = null;
        this.sdk = null;
        this.initialized = false;
        this.initPromise = null;
    }
    
    async initialize(onProgress) {
        if (this.initialized) return this.sdk;
        
        if (this.initPromise) return this.initPromise;
        
        this.initPromise = new Promise((resolve, reject) => {
            // Check if SDK iframe already exists
            let iframe = document.getElementById('sdk-iframe');
            
            if (!iframe) {
                // Create hidden iframe to hold SDK
                iframe = document.createElement('iframe');
                iframe.id = 'sdk-iframe';
                iframe.style.display = 'none';
                iframe.src = '/sdk-loader.html';
                document.body.appendChild(iframe);
            }
            
            this.iframe = iframe;
            
            // Listen for messages from iframe
            const messageHandler = (event) => {
                if (event.source !== iframe.contentWindow) return;
                
                const { type, data } = event.data;
                
                switch (type) {
                    case 'sdk-ready':
                        this.initialized = true;
                        window.removeEventListener('message', messageHandler);
                        resolve(true);
                        break;
                        
                    case 'sdk-progress':
                        if (onProgress) {
                            onProgress(data.percent, data.text);
                        }
                        break;
                        
                    case 'sdk-error':
                        window.removeEventListener('message', messageHandler);
                        reject(new Error(data.error));
                        break;
                }
            };
            
            window.addEventListener('message', messageHandler);
            
            // Check if iframe is already loaded
            if (iframe.contentWindow && iframe.contentWindow.sdkReady) {
                this.initialized = true;
                window.removeEventListener('message', messageHandler);
                resolve(true);
            }
        });
        
        return this.initPromise;
    }
    
    async execute(method, args) {
        if (!this.initialized) {
            await this.initialize();
        }
        
        return new Promise((resolve, reject) => {
            const id = Math.random().toString(36).substr(2, 9);
            
            const messageHandler = (event) => {
                if (event.source !== this.iframe.contentWindow) return;
                if (event.data.id !== id) return;
                
                window.removeEventListener('message', messageHandler);
                
                if (event.data.type === 'result') {
                    resolve(event.data.result);
                } else if (event.data.type === 'error') {
                    reject(new Error(event.data.error));
                }
            };
            
            window.addEventListener('message', messageHandler);
            
            this.iframe.contentWindow.postMessage({
                type: 'execute',
                id,
                method,
                args
            }, '*');
        });
    }
}

// Export singleton
export const sdkManager = new SdkManager();