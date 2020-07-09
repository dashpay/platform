# Quick start

### ES5/ES6 via NPM

In order to use this library in Node, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type in your terminal :

```sh
npm install @dashevo/dapi-client
```

### CDN Standalone

For browser usage, you can also directly rely on unpkg :

```
<script src="https://unpkg.com/@dashevo/dapi-client"></script>
```

## Initialization

```js
const DAPIClient = require('@dashevo/dapi-client');
const client = new DAPIClient();

(async () => {
  const bestBlockHash = await client.core.getBestBlockHash();
  console.log(bestBlockHash);
})();
```

## Quicknotes

This package allows you to fetch & send information from both the payment chain (layer 1) and the application chain (layer 2, a.k.a Platform chain).
