# Pikkr

JSON parser which picks up values directly without performing tokenization

## Abstract

Pikkr is a JSON parser which picks up values directly without performing tokenization. This JSON parser is implemented based on [Y. Li, N. R. Katsipoulakis, B. Chandramouli, J. Goldstein, and D. Kossmann. Mison: a fast JSON parser for data analytics. In *VLDB*, 2017](http://www.vldb.org/pvldb/vol10/p1118-li.pdf).

This JSON parser extracts values from a JSON record without using finite state machines (FSMs) and performing tokenization. It parses JSON records in the following procedures:

1. [Indexing] Creates an index which maps logical locations of queried fields to their physical locations by using SIMD instructions and bit manipulation.
2. [Basic parsing] Finds values of queried fields by scanning a JSON record using the index and learns their logical locations (i.e. pattern of the JSON structure) in the early stages.
3. [Speculative parsing] Speculates logical locations of queried fields by using the learned result information, jumps directly to their physical locations and extracts values in the later stages. Fallbacks to basic parsing if the speculation fails.

This JSON parser performs well when there are a limited number of different JSON structural variants in a JSON data stream or JSON collection and that is a common case in data analytics field.

Please read the paper mentioned in the opening paragraph for the details of the JSON parsing algorithm.

## Performance

## Example

## Restrictions
