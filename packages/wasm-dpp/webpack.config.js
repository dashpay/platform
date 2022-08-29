const path = require('path');

module.exports = {
    entry: './index.ts',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'index.js',
        library: {
            type: 'umd'
        },
        publicPath: '',
        // This is needed to prevent ReferenceError: self is not defined,
        // as webpack names global object "self" for some reason
        globalObject: 'this'
    },
    plugins: [
        // // Have this example work in Edge which doesn't ship `TextEncoder` or
        // // `TextDecoder` at this time.
        // new webpack.ProvidePlugin({
        //     TextDecoder: ['text-encoding', 'TextDecoder'],
        //     TextEncoder: ['text-encoding', 'TextEncoder']
        // })
    ],
    mode: 'development',
    // experiments: {
    //     asyncWebAssembly: true
    // },
    // resolve: {
    //     extensions: ['.wasm']
    // },
    // module: {
    //     rules: [{
    //         test: /\.wasm$/,
    //         type: "asset/inline",
    //     }],
    // }
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                use: 'ts-loader',
                exclude: /node_modules/,
            },
        ],
    },
    resolve: {
        extensions: ['.tsx', '.ts', '.js'],
    },
};
