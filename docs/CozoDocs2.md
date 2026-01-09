System ops
System ops start with a double-colon :: and must appear alone in a script. In the following, we explain what each system op does, and the arguments they expect.

Explain
::explain { <QUERY> }
A single query is enclosed in curly braces. Query options are allowed but ignored. The query is not executed, but its query plan is returned instead. Currently, there is no specification for the return format, but you can decipher the result after reading Query execution.

Ops for stored relations
::relations
List all stored relations in the database

::columns <REL_NAME>
List all columns for the stored relation <REL_NAME>.

::indices <REL_NAME>
List all indices for the stored relation <REL_NAME>.

::describe <REL_NAME> <DESCRIPTION>?
Describe the stored relation <REL_NAME> and store it in the metadata. If <DESCRIPTION> is given, it is stored as the description, otherwise the existing description is removed. The description can be shown with ::relations. It serves as the documentation and signpost for humans and AI.

::remove <REL_NAME> (, <REL_NAME>)*
Remove stored relations. Several can be specified, joined by commas.

::rename <OLD_NAME> -> <NEW_NAME> (, <OLD_NAME> -> <NEW_NAME>)*
Rename stored relation <OLD_NAME> into <NEW_NAME>. Several may be specified, joined by commas.

::index ...
Manage indices. See Stored relations and transactions for more details.

::hnsw ...
Manage HNSW indices. See Proximity searches for more details.

::show_triggers <REL_NAME>
Display triggers associated with the stored relation <REL_NAME>.

::set_triggers <REL_NAME> ...
Set triggers for the stored relation <REL_NAME>. This is explained in more detail in Stored relations and transactions.

::access_level <ACCESS_LEVEL> <REL_NAME> (, <REL_NAME>)*
Sets the access level of <REL_NAME> to the given level. The levels are:

normal allows everything,

protected disallows ::remove and :replace,

read_only additionally disallows any mutations and setting triggers,

hidden additionally disallows any data access (metadata access via ::relations, etc., are still allowed).

The access level functionality is to protect data from mistakes of the programmer, not from attacks by malicious parties.

Monitor and kill
::running
Display running queries and their IDs.

::kill <ID>
Kill a running query specified by <ID>. The ID may be obtained by ::running.

Maintenance
::compact
Instructs Cozo to run a compaction job. Compaction makes the database smaller on disk and faster for read queries.

Types
Runtime types
Values in Cozo have the following runtime types:

Null

Bool

Number

String

Bytes

Uuid

List

Vector

Json

Validity

Number can be Float (double precision) or Int (signed, 64 bits). Cozo will auto-promote Int to Float when necessary.

List can contain any number of mixed-type values, including other lists.

Vector have fixed length and contain floats. There are two versions: F32 vectors and F64 vectors.

Cozo sorts values according to the above order, e.g. null is smaller than true, which is in turn smaller than the list [].

Within each type values are compared according to:

false < true;

-1 == -1.0 < 0 == 0.0 < 0.5 == 0.5 < 1 == 1.0;

Lists are ordered lexicographically by their elements;

Bytes are compared lexicographically;

Strings are compared lexicographically by their UTF-8 byte representations;

UUIDs are sorted in a way that UUIDv1 with similar timestamps are near each other. This is to improve data locality and should be considered an implementation detail. Depending on the order of UUID in your application is not recommended.

Json values are compared by their string representation, which is a bit arbitrary and you should not rely on the order.

Validity is introduced for the sole purpose of enabling time travel queries.

Warning

1 == 1.0 evaluates to true, but 1 and 1.0 are distinct values, meaning that a relation can contain both as keys according to set semantics. This is especially confusing when using JavaScript, which converts all numbers to float, and python, which does not show a difference between the two when printing. Using floating point numbers in keys is not recommended if the rows are accessed by these keys (instead of accessed by iteration).

Literals
The standard notations null for the type Null, false and true for the type Bool are used.

Besides the usual decimal notation for signed integers, you can prefix a number with 0x or -0x for hexadecimal representation, with 0o or -0o for octal, or with 0b or -0b for binary. Floating point numbers include the decimal dot (may be trailing), and may be in scientific notation. All numbers may include underscores _ in their representation for clarity. For example, 299_792_458 is the speed of light in meters per second.

Strings can be typed in the same way as they do in JSON using double quotes "", with the same escape rules. You can also use single quotes '' in which case the roles of double quotes and single quotes are switched. There is also a “raw string” notation:

___"I'm a raw string"___
A raw string starts with an arbitrary number of underscores, and then a double quote. It terminates when followed by a double quote and the same number of underscores. Everything in between is interpreted exactly as typed, including any newlines. By varying the number of underscores, you can represent any string without quoting.

There is no literal representation for Bytes or Uuid. Use the appropriate functions to create them. If you are inserting data into a stored relation with a column specified to contain bytes or UUIDs, auto-coercion will kick in and use decode_base64 and to_uuid for conversion.

Lists are items enclosed between square brackets [], separated by commas. A trailing comma is allowed after the last item.

There are no literal representations for Vector or Validity. Use the function vec to convert a list to a vector.

Json objects are enclosed between curly brackets {}, with key-value pairs separated by commas. For all other Json subtypes, use the function json to convert normal values to them.

Column types
The following atomic types can be specified for columns in stored relations:

Int

Float

Bool

String

Bytes

Uuid

Json

Validity

There is no Null type. Instead, if you put a question mark after a type, it is treated as nullable, meaning that it either takes value in the type or is null.

Two composite types are available. A homogeneous list is specified by square brackets, with the inner type in between, like this: [Int]. You may optionally specify how many elements are expected, like this: [Int; 10]. A heterogeneous list, or a tuple, is specified by round brackets, with the element types listed by position, like this: (Int, Float, String). Tuples always have fixed lengths.

Vectors are also valid as component types and is written in the syntax <F32; 1024> for a 1024-element F32 vector. The type can also be F64.

A special type Any can be specified, allowing all values except null. If you want to allow null as well, use Any?. Composite types may contain other composite types or Any types as their inner types.

Query execution
Databases often consider how queries are executed an implementation detail hidden behind an abstraction barrier that users need not care about, so that databases can utilize query optimizers to choose the best query execution plan regardless of how the query was originally written. This abstraction barrier is leaky, however, since bad query execution plans invariably occur, and users need to “reach behind the curtain” to fix performance problems, which is a difficult and tiring task. The problem becomes more severe the more joins a query contains, and graph queries tend to contain a large number of joins.

So in Cozo we take the pragmatic approach and make query execution deterministic and easy to tell from how the query was written. The flip side is that we demand the user to know what is the best way to store their data, which is in general less demanding than coercing the query optimizer. Then, armed with knowledge of this chapter, writing efficient queries is easy.

Disjunctive normal form
Evaluation starts by canonicalizing inline rules into disjunction normal form, i.e., a disjunction of conjunctions, with any negation pushed to the innermost level. Each clause of the outmost disjunction is then treated as a separate rule. The consequence is that the safety rule may be violated even though textually every variable in the head occurs in the body. As an example:

rule[a, b] := rule1[a] or rule2[b]
is a violation of the safety rule since it is rewritten into two rules, each of which is missing a different binding.

Stratification
The next step in the processing is stratification. It begins by making a graph of the named rules, with the rules themselves as nodes, and a link is added between two nodes when one of the rules applies the other. This application is through atoms for inline rules, and input relations for fixed rules.

Next, some of the links are labelled stratifying:

when an inline rule applies another rule through negation,

when an inline rule applies another inline rule (not itself) that contains aggregations,

when an inline rule applies itself and it has non-semi-lattice,

when an inline rule applies another rule which is a fixed rule,

when a fixed rule has another rule as an input relation.

The strongly connected components of the graph of rules are then determined and tested, and if it found that some strongly connected component contains a stratifying link, the graph is deemed unstratifiable, and the execution aborts. Otherwise, Cozo will topologically sort the strongly connected components to determine the strata of the rules: rules within the same stratum are logically executed together, and no two rules within the same stratum can have a stratifying link between them. In this process, Cozo will merge the strongly connected components into as few supernodes as possible while still maintaining the restriction on stratifying links. The resulting strata are then passed on to be processed in the next step.

You can see the stratum number assigned to rules by using the ::explain system op.

Magic set rewrites
Within each stratum, the input rules are rewritten using the technique of magic sets. This rewriting ensures that the query execution does not waste time calculating results that are later simply discarded. As an example, consider:

reachable[a, b] := link[a, n]
reachable[a, b] := reachable[a, c], link[c, b]
?[r] := reachable['A', r]
Without magic set rewrites, the whole reachable relation is generated first, then most of them are thrown away, keeping only those starting from 'A'. Magic set rewriting avoids this problem. You can see the result of the rewriting using ::explain. The rewritten query is guaranteed to yield the same relation for ?, and will in general yield fewer intermediate rows.

The rewrite currently only applies to inline rules without aggregations.

Semi-naïve evaluation
Now each stratum contains either a single fixed rule or a set of inline rules. The single fixed rules are executed by running their specific implementations. For the inline rules, each of them is assigned an output relation. Assuming we know how to evaluate each rule given all the relations it depends on, the semi-naïve algorithm can now be applied to the rules to yield all output rows.

The semi-naïve algorithm is a bottom-up evaluation strategy, meaning that it tries to deduce all facts from a set of given facts.

Note

By contrast, top-down strategies start with stated goals and try to find proof for the goals. Bottom-up strategies have many advantages over top-down ones when the whole output of each rule is needed, but may waste time generating unused facts if only some of the output is kept. Magic set rewrites are introduced to eliminate precisely this weakness.

Ordering of atoms
The compiler reorders the atoms in the body of the inline rules, and then the atoms are evaluated.

After conversion to disjunctive normal forms, each atom can only be one of the following:

an explicit unification,

applying a rule or a stored relation,

an expression, which should evaluate to a boolean,

a negation of an application.

The first two cases may introduce fresh bindings, whereas the last two cannot. The reordering make all atoms that introduce new bindings stay where they are, whereas all atoms that do not introduce new bindings are moved to the earliest possible place where all their bindings are bound. All atoms that introduce bindings correspond to joining with a pre-existing relation followed by projections in relational algebra, and all atoms that do not correspond to filters. By applying filters as early as possible, we minimize the number of rows before joining them with the next relation.

When writing the body of rules, we should aim to minimize the total number of rows generated. A strategy that works almost in all cases is to put the most restrictive atoms which generate new bindings first.

Evaluating atoms
We now explain how a single atom which generates new bindings is processed.

For unifications, the right-hand side, an expression with all variables bound, is simply evaluated, and the result is joined to the current relation (as in a map-cat operation in functional languages).

Rules or stored relations are conceptually trees, with composite keys sorted lexicographically. The complexity of their applications in atoms is therefore determined by whether the bound variables and constants in the application bindings form a key prefix. For example, the following application:

a_rule['A', 'B', c]
with c unbound, is very efficient, since this corresponds to a prefix scan in the tree with the key prefix ['A', 'B'], whereas the following application:

a_rule[a, 'B', 'C']
where a is unbound, is very expensive, since we must do a full scan. On the other hand, if a is bound, then this is only a logarithmic-time existence check.

For stored relations, you need to check its schema for the order of keys to deduce the complexity. The system op ::explain may also give you some information.

Rows are generated in a streaming fashion, meaning that relation joins proceed as soon as one row is available, and do not wait until the whole relation is generated.

Early stopping
For the entry rule ?, if :limit is specified as a query option, a counter is used to monitor how many valid rows are already generated. If enough rows are generated, the query stops. This only works when the entry rule is inline and you do not specify :order.

Tips for writing queries
Dealing with nulls
Cozo is strict about types. A simple query such as:

?[a] := *rel[a, b], b > 0
will throw if some of the b is null: comparisons can only be made between values of the same type. The solution is that you may decide to consider any null values to be equivalent to some default values:

?[a] := *rel[a, b], (b ~ -1) > 0
here ~ is the coalesce operator. The parentheses are not necessary, but it reads better this way.

You can also check for null explicitly:

?[a] := *rel[a, b], if(is_null(b), false, b > 0)
cond is also helpful in this case.

How to join relations
Suppose we have the following relation:

:create friend {fr, to}
Let’s say we want to find Alice’s friends’ friends’ friends’ friends’ friends. One way to write this is:

?[who] := *friends{fr: 'Alice', to: f1},
          *friends{fr: f1, to: f2},
          *friends{fr: f2, to: f3},
          *friends{fr: f3, to: f4},
          *friends{fr: f4, to: who}
Another way is:

f1[who] := *friends{fr: 'Alice', to: who}
f2[who] := f1[fr], *friends{fr, to: who}
f3[who] := f2[fr], *friends{fr, to: who}
f4[who] := f3[fr], *friends{fr, to: who}
?[who] := f4[fr], *friends{fr, to: who}
These two queries yield identical values. But on real networks, where loops abound, the second way of writing executes exponentially faster than the first. Why? Because of set semantics in relations, the second way of writing deduplicates at every turn, whereas the first way of writing builds up all paths to the final layer of friends. In fact, even if there are no duplicates, the second version may still be faster, because in Cozo rules run in parallel whenever allowed by semantics and available resources.

The moral of the story is, always prefer to break your query into smaller rules. It usually reads better, and unlike in some other databases, it almost always executes faster in Cozo as well. But for this particular case, in which the query is largely recursive, prefer to make it a recursive relation:

f_n[who, min(layer)] := *friends{fr: 'Alice', to: who}, layer = 1
f_n[who, min(layer)] := f_n[fr, last_layer], *friends{fr, to: who}, layer = last_layer + 1, layer <= 5
?[who] := f_n[who, 5]
The condition layer <= 5 is necessary to ensure termination.

Are there any situations where the first way of writing is acceptable? Yes:

?[who] := *friends{fr: 'Alice', to: f1},
          *friends{fr: f1, to: f2},
          *friends{fr: f2, to: f3},
          *friends{fr: f3, to: f4},
          *friends{fr: f4, to: who}
:limit 1
in this case, we stop at the first path, and this way of writing avoids the overhead of multiple rules and is perhaps very slightly faster.

Also, if you want to count the different paths, you must write:

?[count(who)] := *friends{fr: 'Alice', to: f1},
                 *friends{fr: f1, to: f2},
                 *friends{fr: f2, to: f3},
                 *friends{fr: f3, to: f4},
                 *friends{fr: f4, to: who}
The multiple-rules way of writing gives wrong results due to set semantics. Due to the presence of the aggregation count, this query only keeps a single path in memory at any instant, so it won’t blow up your memory even on web-scale data.

Functions and operators
Functions can be used to build expressions.

All functions except those that extract the current time and those having names starting with rand_ are deterministic.

Non-functions
Functions must take in expressions as arguments, evaluate each argument in turn, and then evaluate its implementation to produce a value that can be used in an expression. We first describe constructs that look like, but are not functions.

These are language constucts that return Horn clauses instead of expressions:

var = expr unifies expr with var. Different from expr1 == expr2.

not clause negates a Horn clause clause. Different from !expr or negate(expr).

clause1 or clause2 connects two Horn-clauses by disjunction. Different from or(expr1, expr2).

clause1 and clause2 connects two Horn-clauses by conjunction. Different from and(expr1, expr2).

clause1, clause2 connects two Horn-clauses by conjunction.

For the last three, or binds more tightly from and, which in turn binds more tightly than ,: and and , are identical in every aspect except their binding powers.

These are constructs that return expressions:

if(a, b, c) evaluates a, and if the result is true, evaluate b and returns its value, otherwise evaluate c and returns its value. a must evaluate to a boolean.

if(a, b) same as if(a, b, null)

cond(a1, b1, a2, b2, ...) evaluates a1, if the results is true, returns the value of b1, otherwise continue with a2 and b2. An even number of arguments must be given and the a``s must evaluate to booleans. If all ``a``s are ``false, null is returned. If you want a catch-all clause at the end, put true as the condition.

Operators representing functions
Some functions have equivalent operator forms, which are easier to type and perhaps more familiar. First the binary operators:

a && b is the same as and(a, b)

a || b is the same as or(a, b)

a ^ b is the same as pow(a, b)

a ++ b is the same as concat(a, b)

a + b is the same as add(a, b)

a - b is the same as sub(a, b)

a * b is the same as mul(a, b)

a / b is the same as div(a, b)

a % b is the same as mod(a, b)

a >= b is the same as ge(a, b)

a <= b is the same as le(a, b)

a > b is the same as gt(a, b)

a < b is the same as le(a, b)

a == b is the same as eq(a, b)

a != b is the same as neq(a, b)

a ~ b is the same as coalesce(a, b)

a -> b is the same as maybe_get(a, b)

These operators have precedence as follows (the earlier rows binds more tightly, and within the same row operators have equal binding power):

->

~

^

*, /

+, -, ++

%

==, !=

>=, <=, >, <

&&

||

With the exception of ^, all binary operators are left associative: a / b / c is the same as (a / b) / c. ^ is right associative: a ^ b ^ c is the same as a ^ (b ^ c).

And the unary operators are:

-a is the same as minus(a)

!a is the same as negate(a)

Function applications using parentheses bind the tightest, followed by unary operators, then binary operators.

Equality and Comparisons
eq(x, y)
Equality comparison. The operator form is x == y. The two arguments of the equality can be of different types, in which case the result is false.

neq(x, y)
Inequality comparison. The operator form is x != y. The two arguments of the equality can be of different types, in which case the result is true.

gt(x, y)
Equivalent to x > y

ge(x, y)
Equivalent to x >= y

lt(x, y)
Equivalent to x < y

le(x, y)
Equivalent to x <= y

Note

The four comparison operators can only compare values of the same runtime type. Integers and floats are of the same type Number.

max(x, ...)
Returns the maximum of the arguments. Can only be applied to numbers.

min(x, ...)
Returns the minimum of the arguments. Can only be applied to numbers.

Boolean functions
and(...)
Variadic conjunction. For binary arguments it is equivalent to x && y.

or(...)
Variadic disjunction. For binary arguments it is equivalent to x || y.

negate(x)
Negation. Equivalent to !x.

assert(x, ...)
Returns true if x is true, otherwise will raise an error containing all its arguments as the error message.

Mathematics
add(...)
Variadic addition. The binary version is the same as x + y.

sub(x, y)
Equivalent to x - y.

mul(...)
Variadic multiplication. The binary version is the same as x * y.

div(x, y)
Equivalent to x / y.

minus(x)
Equivalent to -x.

pow(x, y)
Raises x to the power of y. Equivalent to x ^ y. Always returns floating number.

sqrt(x)
Returns the square root of x.

mod(x, y)
Returns the remainder when x is divided by y. Arguments can be floats. The returned value has the same sign as x. Equivalent to x % y.

abs(x)
Returns the absolute value.

signum(x)
Returns 1, 0 or -1, whichever has the same sign as the argument, e.g. signum(to_float('NEG_INFINITY')) == -1, signum(0.0) == 0, but signum(-0.0) == -1. Returns NAN when applied to NAN.

floor(x)
Returns the floor of x.

ceil(x)
Returns the ceiling of x.

round(x)
Returns the nearest integer to the argument (represented as Float if the argument itself is a Float). Round halfway cases away from zero. E.g. round(0.5) == 1.0, round(-0.5) == -1.0, round(1.4) == 1.0.

exp(x)
Returns the exponential of the argument, natural base.

exp2(x)
Returns the exponential base 2 of the argument. Always returns a float.

ln(x)
Returns the natual logarithm.

log2(x)
Returns the logarithm base 2.

log10(x)
Returns the logarithm base 10.

sin(x)
The sine trigonometric function.

cos(x)
The cosine trigonometric function.

tan(x)
The tangent trigonometric function.

asin(x)
The inverse sine.

acos(x)
The inverse cosine.

atan(x)
The inverse tangent.

atan2(x, y)
The inverse tangent atan2 by passing x and y separately.

sinh(x)
The hyperbolic sine.

cosh(x)
The hyperbolic cosine.

tanh(x)
The hyperbolic tangent.

asinh(x)
The inverse hyperbolic sine.

acosh(x)
The inverse hyperbolic cosine.

atanh(x)
The inverse hyperbolic tangent.

deg_to_rad(x)
Converts degrees to radians.

rad_to_deg(x)
Converts radians to degrees.

haversine(a_lat, a_lon, b_lat, b_lon)
Computes with the haversine formula the angle measured in radians between two points a and b on a sphere specified by their latitudes and longitudes. The inputs are in radians. You probably want the next function when you are dealing with maps, since most maps measure angles in degrees instead of radians.

haversine_deg_input(a_lat, a_lon, b_lat, b_lon)
Same as the previous function, but the inputs are in degrees instead of radians. The return value is still in radians.

If you want the approximate distance measured on the surface of the earth instead of the angle between two points, multiply the result by the radius of the earth, which is about 6371 kilometres, 3959 miles, or 3440 nautical miles.

Note

The haversine formula, when applied to the surface of the earth, which is not a perfect sphere, can result in an error of less than one percent.

Vector functions
Now that mathematical functions that operate on floats can also take vectors as arguments, and apply the operation element-wise.

vec(l, type?)
Takes a list of numbers and returns a vector.

Defaults to 32-bit float vectors. If you want to use 64-bit float vectors, pass 'F64' as the second argument.

rand_vec(n, type?)
Returns a vector of n random numbers between 0 and 1.

Defaults to 32-bit float vectors. If you want to use 64-bit float vectors, pass 'F64' as the second argument.

l2_normalize(v)
Takes a vector and returns a vector with the same direction but length 1, normalized using L2 norm.

l2_dist(u, v)
Takes two vectors and returns the distance between them, using squared L2 norm: d = sum((ui-vi)^2).

ip_dist(u, v)
Takes two vectors and returns the distance between them, using inner product: d = 1 - sum(ui*vi).

cos_dist(u, v)
Takes two vectors and returns the distance between them, using cosine distance: d = 1 - sum(ui*vi) / (sqrt(sum(ui^2)) * sqrt(sum(vi^2))).

Json funcitons
json(x)
Converts any value to a Json value. This function is idempotent and never fails.

is_json(x)
Returns true if the argument is a Json value, false otherwise.

json_object(k1, v1, ...)
Convert a list of key-value pairs to a Json object.

dump_json(x)
Convert a Json value to its string representation.

parse_json(x)
Parse a string to a Json value.

get(json, idx, default?)
Returns the element at index idx in the Json json.

idx may be a string (for indexing objects), a number (for indexing arrays), or a list of strings and numbers (for indexing deep structures).

Raises an error if the requested element cannot be found, unless default is specified, in which cast default is returned.

maybe_get(json, idx)
Returns the element at index idx in the Json json. Same as get(json, idx, null). The shorthand is json->idx.

set_json_path(json, path, value)
Set the value at the given path in the given Json value. The path is a list of keys of strings (for indexing objects) or numbers (for indexing arrays). The value is converted to Json if it is not already a Json value.

remove_json_path(json, path)
Remove the value at the given path in the given Json value. The path is a list of keys of strings (for indexing objects) or numbers (for indexing arrays).

json_to_scalar(x)
Convert a Json value to a scalar value if it is a null, boolean, number or string, and returns the argument unchanged otherwise.

concat(x, y, ...)
Concatenate (deep-merge) Json values. It is equivalent to the operator form x ++ y ++ ...

The concatenation of two Json arrays is the concatenation of the two arrays. The concatenation of two Json objects is the deep-merge of the two objects, meaning that their key-value pairs are combined, with any pairs that appear in both left and right having their values deep-merged. For all other cases, the right value wins.

String functions
length(str)
Returns the number of Unicode characters in the string.

Can also be applied to a list or a byte array.

Warning

length(str) does not return the number of bytes of the string representation. Also, what is returned depends on the normalization of the string. So if such details are important, apply unicode_normalize before length.

concat(x, ...)
Concatenates strings. Equivalent to x ++ y in the binary case.

Can also be applied to lists.

str_includes(x, y)
Returns true if x contains the substring y, false otherwise.

lowercase(x)
Convert to lowercase. Supports Unicode.

uppercase(x)
Converts to uppercase. Supports Unicode.

trim(x)
Removes whitespace from both ends of the string.

trim_start(x)
Removes whitespace from the start of the string.

trim_end(x)
Removes whitespace from the end of the string.

starts_with(x, y)
Tests if x starts with y.

Tip

starts_with(var, str) is preferred over equivalent (e.g. regex) conditions, since the compiler may more easily compile the clause into a range scan.

ends_with(x, y)
tests if x ends with y.

unicode_normalize(str, norm)
Converts str to the normalization specified by norm. The valid values of norm are 'nfc', 'nfd', 'nfkc' and 'nfkd'.

chars(str)
Returns Unicode characters of the string as a list of substrings.

from_substrings(list)
Combines the strings in list into a big string. In a sense, it is the inverse function of chars.

Warning

If you want substring slices, indexing strings, etc., first convert the string to a list with chars, do the manipulation on the list, and then recombine with from_substring.

List functions
list(x, ...)
Constructs a list from its argument, e.g. list(1, 2, 3). Equivalent to the literal form [1, 2, 3].

is_in(el, list)
Tests the membership of an element in a list.

first(l)
Extracts the first element of the list. Returns null if given an empty list.

last(l)
Extracts the last element of the list. Returns null if given an empty list.

get(l, n, default?)
Returns the element at index n in the list l. Raises an error if the access is out of bounds, unless default is specified, in which cast default is returned. Indices start with 0.

maybe_get(l, n)
Returns the element at index n in the list l. Same as get(l, n, null). The shorthand is l->n.

length(list)
Returns the length of the list.

Can also be applied to a string or a byte array.

slice(l, start, end)
Returns the slice of list between the index start (inclusive) and end (exclusive). Negative numbers may be used, which is interpreted as counting from the end of the list. E.g. slice([1, 2, 3, 4], 1, 3) == [2, 3], slice([1, 2, 3, 4], 1, -1) == [2, 3].

concat(x, ...)
Concatenates lists. The binary case is equivalent to x ++ y.

Can also be applied to strings.

prepend(l, x)
Prepends x to l.

append(l, x)
Appends x to l.

reverse(l)
Reverses the list.

sorted(l)
Sorts the list and returns the sorted copy.

chunks(l, n)
Splits the list l into chunks of n, e.g. chunks([1, 2, 3, 4, 5], 2) == [[1, 2], [3, 4], [5]].

chunks_exact(l, n)
Splits the list l into chunks of n, discarding any trailing elements, e.g. chunks([1, 2, 3, 4, 5], 2) == [[1, 2], [3, 4]].

windows(l, n)
Splits the list l into overlapping windows of length n. e.g. windows([1, 2, 3, 4, 5], 3) == [[1, 2, 3], [2, 3, 4], [3, 4, 5]].

union(x, y, ...)
Computes the set-theoretic union of all the list arguments.

intersection(x, y, ...)
Computes the set-theoretic intersection of all the list arguments.

difference(x, y, ...)
Computes the set-theoretic difference of the first argument with respect to the rest.

Binary functions
length(bytes)
Returns the length of the byte array.

Can also be applied to a list or a string.

bit_and(x, y)
Calculate the bitwise and. The two bytes must have the same lengths.

bit_or(x, y)
Calculate the bitwise or. The two bytes must have the same lengths.

bit_not(x)
Calculate the bitwise not.

bit_xor(x, y)
Calculate the bitwise xor. The two bytes must have the same lengths.

pack_bits([...])
packs a list of booleans into a byte array; if the list is not divisible by 8, it is padded with false.

unpack_bits(x)
Unpacks a byte array into a list of booleans.

encode_base64(b)
Encodes the byte array b into the Base64-encoded string.

Note

encode_base64 is automatically applied when output to JSON since JSON cannot represent bytes natively.

decode_base64(str)
Tries to decode the str as a Base64-encoded byte array.

Type checking and conversions
coalesce(x, ...)
Returns the first non-null value; coalesce(x, y) is equivalent to x ~ y.

to_string(x)
Convert x to a string: the argument is unchanged if it is already a string, otherwise its JSON string representation will be returned.

to_float(x)
Tries to convert x to a float. Conversion from numbers always succeeds. Conversion from strings has the following special cases in addition to the usual string representation:

INF is converted to infinity;

NEG_INF is converted to negative infinity;

NAN is converted to NAN (but don’t compare NAN by equality, use is_nan instead);

PI is converted to pi (3.14159…);

E is converted to the base of natural logarithms, or Euler’s constant (2.71828…).

Converts null and false to 0.0, true to 1.0.

to_int(x)
Converts to an integer. If x is a validity, extracts the timestamp as an integer.

to_unity(x)
Tries to convert x to 0 or 1: null, false, 0, 0.0, "", [], and the empty bytes are converted to 0, and everything else is converted to 1.

to_bool(x)
Tries to convert x to a boolean. The following are converted to false, and everything else is converted to true:

null

false

0, 0.0

"" (empty string)

the empty byte array

the nil UUID (all zeros)

[] (the empty list)

any validity that is a retraction

to_uuid(x)
Tries to convert x to a UUID. The input must either be a hyphenated UUID string representation or already a UUID for it to succeed.

uuid_timestamp(x)
Extracts the timestamp from a UUID version 1, as seconds since the UNIX epoch. If the UUID is not of version 1, null is returned. If x is not a UUID, an error is raised.

is_null(x)
Checks for null.

is_int(x)
Checks for integers.

is_float(x)
Checks for floats.

is_finite(x)
Returns true if x is an integer or a finite float.

is_infinite(x)
Returns true if x is infinity or negative infinity.

is_nan(x)
Returns true if x is the special float NAN. Returns false when the argument is not of number type.

is_num(x)
Checks for numbers.

is_bytes(x)
Checks for bytes.

is_list(x)
Checks for lists.

is_string(x)
Checks for strings.

is_uuid(x)
Checks for UUIDs.

Random functions
rand_float()
Generates a float in the interval [0, 1], sampled uniformly.

rand_bernoulli(p)
Generates a boolean with probability p of being true.

rand_int(lower, upper)
Generates an integer within the given bounds, both bounds are inclusive.

rand_choose(list)
Randomly chooses an element from list and returns it. If the list is empty, it returns null.

rand_uuid_v1()
Generate a random UUID, version 1 (random bits plus timestamp). The resolution of the timestamp part is much coarser on WASM targets than the others.

rand_uuid_v4()
Generate a random UUID, version 4 (completely random bits).

rand_vec(n, type?)
Generates a vector of n random elements. If type is not given, it defaults to F32.

Regex functions
regex_matches(x, reg)
Tests if x matches the regular expression reg.

regex_replace(x, reg, y)
Replaces the first occurrence of the pattern reg in x with y.

regex_replace_all(x, reg, y)
Replaces all occurrences of the pattern reg in x with y.

regex_extract(x, reg)
Extracts all occurrences of the pattern reg in x and returns them in a list.

regex_extract_first(x, reg)
Extracts the first occurrence of the pattern reg in x and returns it. If none is found, returns null.

Regex syntax
Matching one character:

.             any character except new line
\d            digit (\p{Nd})
\D            not digit
\pN           One-letter name Unicode character class
\p{Greek}     Unicode character class (general category or script)
\PN           Negated one-letter name Unicode character class
\P{Greek}     negated Unicode character class (general category or script)
Character classes:

[xyz]         A character class matching either x, y or z (union).
[^xyz]        A character class matching any character except x, y and z.
[a-z]         A character class matching any character in range a-z.
[[:alpha:]]   ASCII character class ([A-Za-z])
[[:^alpha:]]  Negated ASCII character class ([^A-Za-z])
[x[^xyz]]     Nested/grouping character class (matching any character except y and z)
[a-y&&xyz]    Intersection (matching x or y)
[0-9&&[^4]]   Subtraction using intersection and negation (matching 0-9 except 4)
[0-9--4]      Direct subtraction (matching 0-9 except 4)
[a-g~~b-h]    Symmetric difference (matching `a` and `h` only)
[\[\]]        Escaping in character classes (matching [ or ])
Composites:

xy    concatenation (x followed by y)
x|y   alternation (x or y, prefer x)
Repetitions:

x*        zero or more of x (greedy)
x+        one or more of x (greedy)
x?        zero or one of x (greedy)
x*?       zero or more of x (ungreedy/lazy)
x+?       one or more of x (ungreedy/lazy)
x??       zero or one of x (ungreedy/lazy)
x{n,m}    at least n x and at most m x (greedy)
x{n,}     at least n x (greedy)
x{n}      exactly n x
x{n,m}?   at least n x and at most m x (ungreedy/lazy)
x{n,}?    at least n x (ungreedy/lazy)
x{n}?     exactly n x
Empty matches:

^     the beginning of the text
$     the end of the text
\A    only the beginning of the text
\z    only the end of the text
\b    a Unicode word boundary (\w on one side and \W, \A, or \z on the other)
\B    not a Unicode word boundary
Timestamp functions
now()
Returns the current timestamp as seconds since the UNIX epoch. The resolution is much coarser on WASM targets than the others.

format_timestamp(ts, tz?)
Interpret ts as seconds since the epoch and format as a string according to RFC3339. If ts is a validity, its timestamp will be converted to seconds and used.

If a second string argument is provided, it is interpreted as a timezone and used to format the timestamp.

parse_timestamp(str)
Parse str into seconds since the epoch according to RFC3339.

validity(ts_micro, is_assert?)
Returns a validity object with the given timestamp in microseconds. If is_assert is true, the validity will be asserted, otherwise it will be assumed. Defaults to true.

Aggregations
Aggregations in Cozo can be thought of as a function that acts on a stream of values and produces a single value (the aggregate).

There are two kinds of aggregations in Cozo, ordinary aggregations and semi-lattice aggregations. They are implemented differently in Cozo, with semi-lattice aggregations more powerful (only the latter can be used recursively).

The power of semi-lattice aggregations derive from the additional properties they satisfy: a semilattice:

idempotency
the aggregate of a single value a is a itself,

commutativity
the aggregate of a then b is equal to the aggregate of b then a,

associativity
it is immaterial where we put the parentheses in an aggregate application.

In auto-recursive semi-lattice aggregations, there are soundness constraints on what can be done on the bindings coming from the auto-recursive parts within the body of the rule. Usually you do not need to worry about this at all since the obvious ways of using this functionality are all sound, but as for non-termination due to fresh variables introduced by function applications, Cozo does not (and cannot) check for unsoundness in this case.

Semi-lattice aggregations
min(x)
Aggregate the minimum value of all x.

max(x)
Aggregate the maximum value of all x.

and(var)
Aggregate the logical conjunction of the variable passed in.

or(var)
Aggregate the logical disjunction of the variable passed in.

union(var)
Aggregate the unions of var, which must be a list.

intersection(var)
Aggregate the intersections of var, which must be a list.

choice(var)
Returns a non-null value. If all values are null, returns null. Which one is returned is deterministic but implementation-dependent and may change from version to version.

min_cost([data, cost])
The argument should be a list of two elements and this aggregation chooses the list of the minimum cost.

shortest(var)
var must be a list. Returns the shortest list among all values. Ties will be broken non-deterministically.

bit_and(var)
var must be bytes. Returns the bitwise ‘and’ of the values.

bit_or(var)
var must be bytes. Returns the bitwise ‘or’ of the values.

Ordinary aggregations
count(var)
Count how many values are generated for var (using bag instead of set semantics).

count_unique(var)
Count how many unique values there are for var.

collect(var)
Collect all values for var into a list.

unique(var)
Collect var into a list, keeping each unique value only once.

group_count(var)
Count the occurrence of unique values of var, putting the result into a list of lists, e.g. when applied to 'a', 'b', 'c', 'c', 'a', 'c', the results is [['a', 2], ['b', 1], ['c', 3]].

bit_xor(var)
var must be bytes. Returns the bitwise ‘xor’ of the values.

latest_by([data, time])
The argument should be a list of two elements and this aggregation returns the data of the maximum time. This is very similar to min_cost, the differences being that maximum instead of minimum is used, and non-numerical costs are allowed. Only data is returned.

smallest_by([data, cost])
The argument should be a list of two elements and this aggregation returns the data of the minimum cost. Non-numerical costs are allowed, unlike min_cost. The value null for cost are ignored when comparing.

choice_rand(var)
Non-deterministically chooses one of the values of var as the aggregate. Each value the aggregation encounters has the same probability of being chosen.

Note

This version of choice is not a semi-lattice aggregation since it is impossible to satisfy the uniform sampling requirement while maintaining no state, which is an implementation restriction unlikely to be lifted.

Statistical aggregations
mean(x)
The mean value of x.

sum(x)
The sum of x.

product(x)
The product of x.

variance(x)
The sample variance of x.

std_dev(x)
The sample standard deviation of x.

Utilities and algorithms
Fixed rules in CozoScript apply utilities or algorithms.

The algorithms described here are only available if your distribution of Cozo is compiled with the graph-algo feature flag. Currently all prebuilt binaries except WASM are compiled with this flag on.

If you are using the Cozo libraries in Rust, Python or NodeJS, or if you are using the standalone executable, you can also easily define custom fixed rules in the hosting environment: see the respective documentations for how to do it.

Utilities
Constant(data: [...])
Returns a relation containing the data passed in. The constant rule ?[] <- ... is syntax sugar for ?[] <~ Constant(data: ...).

Parameters:
data – A list of lists, representing the rows of the returned relation.

ReorderSort(rel[...], out: [...], sort_by: [...], descending: false, break_ties: false, skip: 0, take: 0)
Sort and then extract new columns of the passed in relation rel.

Parameters:
out (required) – A list of expressions which will be used to produce the output relation. Any bindings in the expressions will be bound to the named positions in rel.

sort_by – A list of expressions which will be used to produce the sort keys. Any bindings in the expressions will be bound to the named positions in rel.

descending – Whether the sorting process should be done in descending order. Defaults to false.

break_ties – Whether ties should be broken, e.g. whether the first two rows with identical sort keys should be given ordering numbers 1 and 2 instead of 1 and 1. Defaults to false.

skip – How many rows to skip before producing rows. Defaults to zero.

take – How many rows at most to produce. Zero means no limit. Defaults to zero.

Returns:
The returned relation, in addition to the rows specified in the parameter out, will have the ordering prepended. The ordering starts at 1.

Tip

This algorithm serves a similar purpose to the global :order, :limit and :offset options, but can be applied to intermediate results. Prefer the global options if it is applied to the final output.

CsvReader(url: ..., types: [...], delimiter: ',', prepend_index: false, has_headers: true)
Read a CSV file from disk or an HTTP GET request and convert the result to a relation.

This utility is not available on WASM targets. In addition, if the feature flag requests is off, only reading from local file is supported.

Parameters:
url (required) – URL for the CSV file. For local file, use file://<PATH_TO_FILE>.

types (required) – A list of strings interpreted as types for the columns of the output relation. If any type is specified as nullable and conversion to the specified type fails, null will be the result. This is more lenient than other functions since CSVs tend to contain lots of bad values.

delimiter – The delimiter to use when parsing the CSV file.

prepend_index – If true, row index will be prepended to the columns.

has_headers – Whether the CSV file has headers. The reader will not interpret the header in any way but will instead simply ignore it.

JsonReader(url: ..., fields: [...], json_lines: true, null_if_absent: false, prepend_index: false)
Read a JSON file for disk or an HTTP GET request and convert the result to a relation.

This utility is not available on WASM targets. In addition, if the feature flag requests is off, only reading from local file is supported.

Parameters:
url (required) – URL for the JSON file. For local file, use file://<PATH_TO_FILE>.

fields (required) – A list of field names, for extracting fields from JSON arrays into the relation.

json_lines – If true, parse the file as lines of JSON objects, each line containing a single object; if false, parse the file as a JSON array containing many objects.

null_if_absent – If a true and a requested field is absent, will output null in its place. If false and the requested field is absent, will throw an error.

prepend_index – If true, row index will be prepended to the columns.

Connectedness algorithms
ConnectedComponents(edges[from, to])
Computes the connected components of a graph with the provided edges.

Returns:
Pairs containing the node index, and its component index.

StronglyConnectedComponent(edges[from, to])
Computes the strongly connected components of a graph with the provided edges.

Returns:
Pairs containing the node index, and its component index.

SCC(...)
See Algo.StronglyConnectedComponent.

MinimumSpanningForestKruskal(edges[from, to, weight?])
Runs Kruskal’s algorithm on the provided edges to compute a minimum spanning forest. Negative weights are fine.

Returns:
Triples containing the from-node, the to-node, and the cost from the tree root to the to-node. Which nodes are chosen to be the roots are non-deterministic. Multiple roots imply the graph is disconnected.

MinimumSpanningTreePrim(edges[from, to, weight?], starting?[idx])
Runs Prim’s algorithm on the provided edges to compute a minimum spanning tree. starting should be a relation producing exactly one node index as the starting node. Only the connected component of the starting node is returned. If starting is omitted, which component is returned is arbitrary.

Returns:
Triples containing the from-node, the to-node, and the cost from the tree root to the to-node.

TopSort(edges[from, to])
Performs topological sorting on the graph with the provided edges. The graph is required to be connected in the first place.

Returns:
Pairs containing the sort order and the node index.

Pathfinding algorithms
ShortestPathBFS(edges[from, to], starting[start_idx], goals[goal_idx])
Runs breadth-first search to determine the shortest path between the starting nodes and the goals. Assumes the graph to be directed and all edges to be of unit weight. Ties will be broken in an unspecified way. If you need anything more complicated, use one of the other algorithms below.

Returns:
Triples containing the starting node, the goal, and a shortest path.

ShortestPathDijkstra(edges[from, to, weight?], starting[idx], goals[idx], undirected: false, keep_ties: false)
Runs Dijkstra’s algorithm to determine the shortest paths between the starting nodes and the goals. Weights, if given, must be non-negative.

Parameters:
undirected – Whether the graph should be interpreted as undirected. Defaults to false.

keep_ties – Whether to return all paths with the same lowest cost. Defaults to false, in which any one path of the lowest cost could be returned.

Returns:
4-tuples containing the starting node, the goal, the lowest cost, and a path with the lowest cost.

KShortestPathYen(edges[from, to, weight?], starting[idx], goals[idx], k: expr, undirected: false)
Runs Yen’s algorithm (backed by Dijkstra’s algorithm) to find the k-shortest paths between nodes in starting and nodes in goals.

Parameters:
k (required) – How many routes to return for each start-goal pair.

undirected – Whether the graph should be interpreted as undirected. Defaults to false.

Returns:
4-tuples containing the starting node, the goal, the cost, and a path with the cost.

BreadthFirstSearch(edges[from, to], nodes[idx, ...], starting?[idx], condition: expr, limit: 1)
Runs breadth first search on the directed graph with the given edges and nodes, starting at the nodes in starting. If starting is not given, it will default to all of nodes, which may be quite a lot to calculate.

Parameters:
condition (required) – The stopping condition, will be evaluated with the bindings given to nodes. Should evaluate to a boolean, with true indicating an acceptable answer was found.

limit – How many answers to produce for each starting nodes. Defaults to 1.

Returns:
Triples containing the starting node, the answer node, and the found path connecting them.

BFS(...)
See Algo.BreadthFirstSearch.

DepthFirstSearch(edges[from, to], nodes[idx, ...], starting?[idx], condition: expr, limit: 1)
Runs depth first search on the directed graph with the given edges and nodes, starting at the nodes in starting. If starting is not given, it will default to all of nodes, which may be quite a lot to calculate.

Parameters:
condition (required) – The stopping condition, will be evaluated with the bindings given to nodes. Should evaluate to a boolean, with true indicating an acceptable answer was found.

limit – How many answers to produce for each starting nodes. Defaults to 1.

Returns:
Triples containing the starting node, the answer node, and the found path connecting them.

DFS(...)
See Algo.DepthFirstSearch.

ShortestPathAStar(edges[from, to, weight], nodes[idx, ...], starting[idx], goals[idx], heuristic: expr)
Computes the shortest path from every node in starting to every node in goals by the A* algorithm.

edges are interpreted as directed, weighted edges with non-negative weights.

Parameters:
heuristic (required) – The search heuristic expression. It will be evaluated with the bindings from goals and nodes. It should return a number which is a lower bound of the true shortest distance from a node to the goal node. If the estimate is not a valid lower-bound, i.e. it over-estimates, the results returned may not be correct.

Returns:
4-tuples containing the starting node index, the goal node index, the lowest cost, and a path with the lowest cost.

Tip

The performance of A* star algorithm heavily depends on how good your heuristic function is. Passing in 0 as the estimate is always valid, but then you really should be using Dijkstra’s algorithm.

Good heuristics usually come about from a metric in the ambient space in which your data live, e.g. spherical distance on the surface of a sphere, or Manhattan distance on a grid. Func.Math.haversine_deg_input could be helpful for the spherical case. Note that you must use the correct units for the distance.

Providing a heuristic that is not guaranteed to be a lower-bound might be acceptable if you are fine with inaccuracies. The errors in the answers are bound by the sum of the margins of your over-estimates.

Community detection algorithms
ClusteringCoefficients(edges[from, to, weight?])
Computes the clustering coefficients of the graph with the provided edges.

Returns:
4-tuples containing the node index, the clustering coefficient, the number of triangles attached to the node, and the total degree of the node.

CommunityDetectionLouvain(edges[from, to, weight?], undirected: false, max_iter: 10, delta: 0.0001, keep_depth?: depth)
Runs the Louvain algorithm on the graph with the provided edges, optionally non-negatively weighted.

Parameters:
undirected – Whether the graph should be interpreted as undirected. Defaults to false.

max_iter – The maximum number of iterations to run within each epoch of the algorithm. Defaults to 10.

delta – How much the modularity has to change before a step in the algorithm is considered to be an improvement.

keep_depth – How many levels in the hierarchy of communities to keep in the final result. If omitted, all levels are kept.

Returns:
Pairs containing the label for a community, and a node index belonging to the community. Each label is a list of integers with maximum length constrained by the parameter keep_depth. This list represents the hierarchy of sub-communities containing the list.

LabelPropagation(edges[from, to, weight?], undirected: false, max_iter: 10)
Runs the label propagation algorithm on the graph with the provided edges, optionally weighted.

Parameters:
undirected – Whether the graph should be interpreted as undirected. Defaults to false.

max_iter – The maximum number of iterations to run. Defaults to 10.

Returns:
Pairs containing the integer label for a community, and a node index belonging to the community.

Centrality measures
DegreeCentrality(edges[from, to])
Computes the degree centrality of the nodes in the graph with the given edges. The computation is trivial, so this should be your first thing to try when exploring new data.

Returns:
4-tuples containing the node index, the total degree (how many edges involve this node), the out-degree (how many edges point away from this node), and the in-degree (how many edges point to this node).

PageRank(edges[from, to, weight?], undirected: false, theta: 0.85, epsilon: 0.0001, iterations: 10)
Computes the PageRank from the given graph with the provided edges, optionally weighted.

Parameters:
undirected – Whether the graph should be interpreted as undirected. Defaults to false.

theta – A number between 0 and 1 indicating how much weight in the PageRank matrix is due to the explicit edges. A number of 1 indicates no random restarts. Defaults to 0.8.

epsilon – Minimum PageRank change in any node for an iteration to be considered an improvement. Defaults to 0.05.

iterations – How many iterations to run. Fewer iterations are run if convergence is reached. Defaults to 20.

Returns:
Pairs containing the node label and its PageRank.

ClosenessCentrality(edges[from, to, weight?], undirected: false)
Computes the closeness centrality of the graph. The input relation represent edges connecting node indices which are optionally weighted.

Parameters:
undirected – Whether the edges should be interpreted as undirected. Defaults to false.

Returns:
Node index together with its centrality.

BetweennessCentrality(edges[from, to, weight?], undirected: false)
Computes the betweenness centrality of the graph. The input relation represent edges connecting node indices which are optionally weighted.

Parameters:
undirected – Whether the edges should be interpreted as undirected. Defaults to false.

Returns:
Node index together with its centrality.

Warning

BetweennessCentrality is very expensive for medium to large graphs. If possible, collapse large graphs into supergraphs by running a community detection algorithm first.

Miscellaneous
RandomWalk(edges[from, to, ...], nodes[idx, ...], starting[idx], steps: 10, weight?: expr, iterations: 1)
Performs random walk on the graph with the provided edges and nodes, starting at the nodes in starting.

Parameters:
steps (required) – How many steps to walk for each node in starting. Produced paths may be shorter if dead ends are reached.

weight – An expression evaluated against bindings of nodes and bindings of edges, at a time when the walk is at a node and choosing between multiple edges to follow. It should evaluate to a non-negative number indicating the weight of the given choice of edge to follow. If omitted, which edge to follow is chosen uniformly.

iterations – How many times walking is repeated for each starting node.

Returns:
Triples containing a numerical index for the walk, the starting node, and the path followed.

Version 0.7: MinHash-LSH near-duplicate indexing, Full-text search (FTS) indexing, Json values and update
Continuing with the empowerment of CozoDB by vector search, This version brings you a few more features!

MinHash-LSH indices
Let’s say you collect news articles from the Internet. There will be duplicates, but these are not exact duplicates. How do you deduplicate them? Simple. Let’s say your article is stored thus:

:create article{id: Int => content: String}
To find the duplicates, you create an LSH index on it:

::lsh create article:lsh {
    extractor: content, 
    tokenizer: Simple, 
    n_gram: 7,
    n_perm: 200,
    target_threshold: 0.7,
}
Now if you do this query:

?[id, content] := ~article:lsh {id, content | query: $q }
then articles with its content about 70% or more similar to the passed-in text in $q will be returned to you.

If you want, you can also mark the duplicates at insertion time. For this, use the following schema:

:create article{id: Int => content: String, dup_for: Int?}
Then at insertion time, use the query:

{
    ?[id, dup_for] := ~article:lsh {id, dup_for | query: $q, k: 1} 
    :create _existing {id, dup_for}
}

%if _existing
    %then {
        ?[id, content, dup_for] := *_existing[eid, edup], 
                                   id = $id, 
                                   content = $content, 
                                   dup_for = edup ~ eid
        :put article {id => content, dup_for}
    }
    %else {
        ?[id, content, dup_for] <- [[$id, $content, null]]
        :put article {id => content, dup_for}
    }
%end
For our own use-case, this achieves about 20x speedup compared to using the equivalent Python library. And we are no longer bound by RAM.

As with vector search, LSH-search integrates seamlessly with Datalog in CozoDB.

Full-text search
After finding the duplicates, what if I want all articles that mentions the word “iPhone”? Using vector search seems such an overkill and may not yield good results. So we have full-text search indices as well!

To apply the FTS index:

::fts create article:fts {
    extractor: content,
    tokenizer: Simple,
    filters: [Lowercase, Stemmer('english'), Stopwords('en')]
}
and it’s ready to be searched:

?[id, content, score] := ~article:fts {id, content | query: $q, bind_score: score }
:order -score
Passing 'iPhone' to $q will give you articles that explicitly mention the iPhone, 'iPhone iPad' will give you articles that mention both, and 'iPhone OR iPad' will give you articles that mention either.

For more information on FTS and LSH, refer to the proximity search chapter.

Json values and update
Now we will have AI commentators analyze and comment on the articles. We will use the following schema:

:create article{id: Int => content: String, dup_for: Int?, comments: Json default {}}
The Json type is newly available. Now let’s say our economics analyzer has produced a report for article 42, in the following Json:

{
    "economic_impact": "The economic impact of ChatGPT has been significant since its introduction. As an advanced language model, ChatGPT has revolutionized various industries and business sectors. It has enhanced customer support services by providing automated and intelligent responses, reducing the need for human intervention. This efficiency has resulted in cost savings for businesses while improving customer satisfaction. Moreover, ChatGPT has been utilized for market research, content generation, and data analysis, empowering organizations to make informed decisions quickly. Overall, ChatGPT has streamlined processes, increased productivity, and created new opportunities, thus positively impacting the economy by driving innovation and growth."
}
To merge this comment into the relation:

?[id, json] := id = $id, *article{id, json: old}, json = old ++ $new_report
:update article {id => json}
Note that with :update, we did not specify the dup_for field, and it will keep whatever its old value.

Next, our political analyzer AI weighs in:

{
    "political_impact": "The political impact of ChatGPT has been a subject of debate and scrutiny. On one hand, ChatGPT has the potential to democratize access to information and empower individuals to engage in political discourse. It can facilitate communication between citizens and government officials, enabling greater transparency and accountability. Additionally, it can assist in analyzing vast amounts of data, helping policymakers make informed decisions. However, concerns have been raised regarding the potential misuse of ChatGPT in spreading disinformation or manipulating public opinion. The technology's ability to generate realistic and persuasive content raises ethical and regulatory challenges, necessitating careful consideration of its deployment to ensure fair and responsible use. As a result, the political impact of ChatGPT remains complex, with both potential benefits and risks to navigate."
}
We just run the same query but with a different $new_report bound.

Now if you query the database for article 42, you will see that its comments contain both reports!

There are many more things you can do with Json values: refer to Functions and operators and Types for more details.

Misc
In this version the semantics of %if in imperative scripts has changed: now a relation is considered truthy as long as it contains any row at all, regardless of the content of its rows. We found the old behaviour confusing in most circumstances.

Happy hacking!