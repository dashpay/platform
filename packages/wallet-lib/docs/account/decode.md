**Usage**: `account.decode(method, encodedValue)`    
**Description**: Allow to decode an encoded value.   
**Notes**: Method allowed right now limited to cbor (used by platform protocol).      

Parameters: 

| parameters        | type   | required       | Description                                      |  
|-------------------|--------|----------------| -------------------------------------------------|
| **method**        | String | yes            | Enter a valid decoding method (one of: ['cbor']) |
| **encodedValue**  | Buffer | yes            | An encoded buffer value                          |

Returns : decoded value (string).
