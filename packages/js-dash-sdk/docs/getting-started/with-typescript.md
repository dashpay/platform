In order to use DashJS with TypeScript.    

Create a index.ts file  

```js
import DashJS from 'dash';
const sdkOpts = {
  network: 'testnet',
  mnemonic: null,// Will generate a new address, you should keep it.
};
const sdk = new DashJS.SDK(sdkOpts);

sdk.isReady().then(()=> console.log('isReady'));
```

Have a following `tsconfig.json` file

```json
{
  "compilerOptions": {
    "module": "commonjs",
    "moduleResolution": "node",
    "esModuleInterop": true
  }
}
```

**Compile:** `tsc -p tsconfig.json`  
**Run:** `node index.js`  
