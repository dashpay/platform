# Dash Platform Rust SDK

This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash Platform along with data models based on the Dash Platform Protocol (DPP), a CRUD interface, and bindings for other technologies such as C.

## Features

- **Abstraction Layer:** Simplifies the usage of Dash Platform.
- **Data Model:** Based on Dash Platform Protocol (DPP).
- **CRUD Interface:** Allows for easy manipulation of data.
- **Technology Bindings:** Includes bindings for other technologies like C.

## `platform` Module: Data Model

The data model in module `platform` consists of Dash Platform Protocol (DPP) objects that are wrapped into SDK wrapper objects using the Newtype design pattern. These SDK objects have an `Sdk` prefix followed by the name of the DPP object. For example, the wrapper for Identity is `SdkIdentity`.

In addition to this, the `TryFrom` trait has been implemented to allow for easy conversion between SDK and DPP objects.

## `crud` Module: CRUD Interface

The crud module provides a comprehensive interface for Create, Read, Update, and Delete (CRUD) operations on the Dash Platform. The module comprises several traits designed to streamline interactions with the platform:

- Readable: This trait is designed for reading data from the Dash Platform. It requires an object's identifier as an SdkQuery parameter that will return exactly one item.
- Listable: This trait allows listing of data from the Dash Platform. It uses the SdkQuery to define the search criteria for the data to be listed.
- Writable: This trait is currently under development. Once completed, it will enable modification of data on the Dash Platform.
