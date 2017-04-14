//Return an array from a list of arguments
Buffer.toArray = function(args){
    let arr = Array.prototype.slice.call(args);
    if(arr.length==1){
        arr = Array.prototype.slice.call(arr[0]);
    }
    return arr;
};
//Return an array where json arguments are parsed into JSON.
//If JSON.parse fails, then it return a buffer in the array
Buffer.toJSON = function(){

    let arr = this.toArray(arguments);
    let arr2 =[];
    // cl(arr.length)
    for(var i=0; i<arr.length; i++){
        if(i==2){

            // cl(arr[2].toString());
        }
        try{
            let msgStringified = arr[i].toString();
            let parsedJson = JSON.parse(msgStringified);
            arr2.push(parsedJson);
        }catch(e){
            // console.error('Not stringified json');
            // cl(arr[i]);
            arr2.push(arr[i]);
        }
    }
    return arr2;
};

/**
 * Attempts to turn a value into a `Buffer`. As input it supports `Buffer`, `String`, `Number`, null/undefined, `BN` and other objects with a `toArray()` method.
 * @param {*} v the value
 */
Buffer.toBuffer = function (v) {
    if (!Buffer.isBuffer(v)) {
        if (Array.isArray(v)) {
            v = Buffer.from(v)
        } else if (typeof v === 'string') {
            if (exports.isHexPrefixed(v)) {
                v = Buffer.from(exports.padToEven(exports.stripHexPrefix(v)), 'hex')
            } else {
                v = Buffer.from(v)
            }
        } else if (typeof v === 'number') {
            v = exports.intToBuffer(v)
        } else if (v === null || v === undefined) {
            v = Buffer.allocUnsafe(0)
        } else if (v.toArray) {
            // converts a BN to a Buffer
            v = Buffer.from(v.toArray())
        } else {
            throw new Error('invalid type')
        }
    }
    return v
}

module.exports = Buffer;