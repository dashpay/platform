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

module.exports = Buffer;