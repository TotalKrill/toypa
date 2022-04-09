# Toy payment engine

A toy payment engine

# Usage

  cargo run -- <inputfile> > <outputfile>

# Implementation

The payment engine only handles disputes on deposit transactions, it also stores all deposits locally 
under each account for retrieval in dispute matters, eating quite a lot of ram on large datasets. 

Theres a fixed point implementation running in the account handling, treating all internal values as integers, 
with the unit of 1/1000th of an amount. Upon serialization or deserialization, they are again transformed to f64 and serialized
mostly because trying to implement a custom serializers and deserializer is always a bit painful