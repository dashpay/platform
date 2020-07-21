## Persistence

Wallet-lib allows the use of a persistence adapter in order to store information fetched via the transporter in order to provide faster loading on later uses.  

This adapter can be useful for multiple cases, for instance: 

- A degraded connectivity: Having stored information on a persistence layer (localStorage, secureStorage,...) would allow a user to still be able to consult his transaction history, UTXO set, balance, prepare and sign a transaction (intended to be broadcasted later, when connectivity is back).  
- Offline Mode: In some conditions, using the wallet-lib on a non-connected device might be a desired feature. The persistence adapter would allow such usage be still providing most of it's feature from it's cache, and therefore, in the example of a transaction signing, the signing would be done on the offline device, while the broadcast would happen on another, connected device.  

When no persistence is set, Wallet-lib will use by default, an In Memory adapter, which won't persist information except in local RAM.  
A message will warn you about this on starting up, and won't be displayed with a properly set adapter.

### Create your own persistence adapter

By just providing a class or instance of a class containing a certain minimal set of methods, one can provide an adapter for various databases, remote services or file storage.  

- `config(props)` - async / optional - When provided, before any execution, this method would be called passing with the following property `name: 'dashevo-wallet-lib'`.

This method intends to allow the preparation of your persistence layer to be ready for further uses (for instance, in a case where your adapter is a database, this would allow to set indexes, and prepare the connection pool).

- `setItem(key, item)` - async / mandatory - This is the method that will be used to set any item to the persistence layer. 

- `getItem(key)` - async / mandatory - This will be called in order to retrieve any item from the persistence layer.
