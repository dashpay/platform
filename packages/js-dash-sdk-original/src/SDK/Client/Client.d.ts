import DAPIClient from "@dashevo/dapi-client"

export declare namespace SDK {
    interface platformOpts {
        client: DAPIClient;
        apps: object;
        state: object;
    }
}
