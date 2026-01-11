Named Entity Extraction with Finite State
Transducers
Diego Alexander Huerfano Villalba∗
, Elizabeth Leon Guzm ´ an´
†
MIDAS Research Group
Department of Computer Engineering
National University of Colombia
Email: ∗dahuerfanov@unal.edu.co, †
eleonguz@unal.edu.co
Abstract—We describe a named entity tagging system that
requires minimal linguistic knowledge and can be applied to
more target languages without substantial changes. The system
is based on the ideas of the Brill’s tagger which makes it really
simple. Using supervised machine learning, we construct a series
of automatons (or transducers) in order to tag a given text. The
final model is composed entirely of automatons and it requires
a lineal time for tagging. It was tested with the Spanish data set
provided in the CoNLL-2002 attaining an overall Fβ=1 measure
of 60%. Also, we present an algorithm for the construction of the
final transducer used to encode all the learned contextual rules.
Index Terms—Algorithm KMP, automaton, named entities,
finite state transducer, local extension of a contextual rule, string
matching, transducer determinization.
I. INTRODUCTION
T
HE task of Named Entity Extraction (NEE) consists
in detection of named entities that appears in a given
text. Named entities are understood to be anything that has a
proper name (people, places, organizations and so on) [1]. The
output obtained form this process is widely used in several
Natural Language Processing (NLP) applications like Question Answering, Machine Translation, Information Retrieval,
Summarization and Topics Detection. This task is carried out
through two main sub-tasks: Named Entity Recognition (NER)
and Classification (NEC) which are often merged in a single
process. This work presents a system based on finite state
transducers to perform NEE where these two sub-tasks are
performed simultaneously.
Approaches to NEE with finite state automatons have been
previously developed. In [2] is proposed a system for NER
in Serbian. Their system is based on handmade finite state
automatons and E-dictionaries recognizing several kinds of
names and temporal and numeric expressions. The overall
performance of this system is a recall of 80% and a precision
of 98%. On the other hand, in [3], it is presented an algorithm
using the approach of Casual-States Spliting Reconstruction
(CSSR) where an automaton is learned from sequential data.
In the end, this automaton is able to recognize named entities
in a text. The system had a recall of 88% and a precision of
89%.
This paper presents a system for the complete NEE task
using finite state transducers learned from annotated texts.
The system is based on the ideas of the Brill’s tagger [4]
in order to learn change rules, then a deterministic transducer
is constructed from these rules. It is also presented the results
and evaluation using the data from the CoNLL-2002 Shared
Task.
II. DESCRIPTION OF THE SYSTEM
The system presented here uses the BIO scheme also used
in the CoNLL-2002 in order to identify four kinds of named
entities: person, location, organization and a last class called
miscellaneous. For tagging a given text we first use a lexical
tagger which only takes into account the form a single word
and its most likely tag according to the training text. This first
stage can be seen as a initialization. Then we infer change
rules to correct those initial tags considering the context which
a single word appears in (words before and after that word).
This is called contextual tagger. This approach was proposed
in [4] for the part-of-speech task. In addition, we encode
the contextual tagger into a single deterministic finite state
transducer allowing to tag the text with a linear complexity.
The construction of such a transducer was proposed in [5],
however, we propose here a new algorithm for the local
extension procedure since we have found that the algorithm
introduced there does not work in all cases.
The training text is divided in two parts. The first one is
used for the lexical tagger so that we can estimate the most
likely tag of every word. The second part is used to infer
change rules allowing us to test which is the best rule.
A. Lexical Tagger
The purpose of this module is to give a initial tag a every
single word in a text. In the same way that Brill proposed
in [4], we have for every word a register of the most likely
tag according to the given training text, which sometimes can
be wrong. We used the structure trie, which can be thought
as a finite state automaton which accepts a finite language,
to store all words and their most likely tag allowing us to
search the tag of a given word in the dictionary in linear time.
Since some words can be unknown, a second trie is used to
store words suffix and its most likely tag, so that we can tag
unknown words using only its suffix (of 4 characters length).
Also, we encode every word in the training text replacing its
capital characters by ’X’, lower case characters by ’x’, digits
by ’d’ and the remaining characters are not changed. In this
way, we can tag unknown words considering only their form
arXiv:2006.11548v1 [cs.CL] 20 Jun 2020
according to known words with similar form. This information
is also held in a new trie.
B. Contextual Tagger
Since the prior initialization can be wrong for some words,
we need to change some initial tags to the correct ones taking
into account the context where each word is. This context is,
in our case, the words tags around one word (we considered up
to two previous tokens and up to two next tokens). As in the
Brill’s tagger, we construct an ordered list of these correction
rules which are applied along all the text in the given order.
An example of the structure of these rules is abt1de → abt2de
where a, b, c, d, t1, t2 can take the value of any possible tags
that we are working with (according to the BIO scheme) and
are considered to be variables for this structure. This example
means that when we find the pattern abt1de in the text, we
change the tag t1 with the tag t2 and the remaining tags in
the pattern are left unchanged. In this way, we are changing
specific tags in the text which are in a specific context.
So as to make the list of contextual rules, we infer each rule
in a stage, where every structures with all possibles values for
each variable are tested with the initialized text and then it is
calculated its score (it is the difference between the number
of correct changes made and the number of incorrect changes
made in comparison with the training test). The rule with the
best score in a stage is chosen and added to the ordered list.
Then we apply it to the initialized text and we continue with
the next stage.
As it is observed in [5], apply these rules may require RCn
elementary steps to tag an input of n words with R contextual
rules requiring at most C tokens of context. Therefore, it is
proposed there a new way to encode all these rules into a
single deterministic finite state transducer allowing to do the
same procedure in n elementary steps, which corresponds to
follow a single path in the transducer according to the tags
that we find when we are processing the text token by token.
In the end, we will have constructed a model which is based
in automatons (or transducers) where all the tagging stage is
performed in linear time.
C. Trigger Words and Punctuation Marks
In addition to the tags proposed in the BIO scheme, we
also used some tags to identify trigger words (prepositions)
which are likely to appear before a named entity like en, de,
por, segun´ . This is the only assumption about the Spanish
language. Also, there is a tag called PUNCTUATION which
covers all the punctuation marks in the text. This was made so
that contextual rules with tags of the BIO scheme apply only
in complete sentences.
III. CONSTRUCTION OF THE DETERMINISTIC FINITE
STATE TRANSDUCER
On overall, the construction of the final transducer, given the
ordered list of contextual rules, is carried out in the following
three steps:
1) For each contextual rule it is constructed the local
extension transducer.
Figure 1: Transducer for the rule aa → ba.
2) The composition operation is applied over the transducers in the same order as their corresponding rules were
in the ordered list. The result of this step is a single
transducer which can be non-deterministic.
3) It is applied the determinization algorithm over the final
transducer.
This method was proposed in [5] for the part-of-speech
tagging. There it is proved that the kind of rules which
come up in this process can be represented by deterministic
finite transducers, therefore the well known determinization
algorithm for transducers finishes in some point. Since the
composition algorithm for transducers is also a standard algorithm in the literature, we will focus on the explanation of the
local extension algorithm.
A. Local Extension of a Rule
In this section we show how to construct the local extension
transducer of a given contextual rule
a1 · · · ant1an+1 · · · an+m → a1 · · · ant2an+1 · · · an+m
where we change t1 by t2. The local extension transducer is
a transducer which will be able to find all the occurrences of
a1 · · · ant1an+1 · · · an+m in a given text and replace them by
a1 · · · ant2an+1 · · · an+m leaving all the other characters outside the occurrences unchanged. In our algorithm we consider
all the occurrences in a text without overlaps giving priority
to the occurrence which appears first.
As we have pointed out before, in [5] it is presented an
algorithm for the same purpose with the difference that they
take a given rule encoded in a transducer and then the algorithm computes the local extension from this given automaton.
However, we have noticed that the given procedure leads to
ambiguities. In the figure 1 it is represented a transducer
encoding the rule aa → ba and in the figure 2 it is shown
the result of their algorithm for this transducer. We can see
that if have the text aaa there are two possible outputs,
which finish in final states: aba with the sequence of states 0,
2, 3, 4, 0 and baa with the sequence of states 0, 3, 4, 0, 2. These
kinds of ambiguities are problematic when the algorithm of
determinization is applied because there is no unique output
for the same sequence of characters. Besides, we have two
occurrences overlapping in the example: aaa and aaa, but,
according to our definition of local extension, we give priority
to the first found occurrence, therefore the only correct output
would be baa. Note that we do not apply the rule to baa in the
last output and obtain bba since we only consider occurrences
without overlaps.
The algorithm presented here to solve this problem is based
on the algorithm of string matching KMP. For string matching
2
Figure 2: Ambiguous local extension for the rule aa → ba.
we can construct a finite automaton that scans the text string
P for all occurrences. [6] presents a method for building such
an automaton which solves the problem in linear time. In our
case, we need to find the pattern P and replace it for a new
text P
0
. According to the form of every contextual rule, the
difference between the texts P and P
0
is only a character, then
the input of the algorithm is the text P, an index 0 ≤ k <
P.length and a character c which means that P
0
is the same
text P but with the character c at the position k.
We adopt the same notation used in [6] and [5]. Given
two strings S1, S2 we denote S1 A S2 if S1 is suffix of
S2 and S1 @ S2 if S1 is prefix of S2. For a given string
S and −1 ≤ k < S.length, Sk = S[0, . . . , k]. Note that
S−1 is the empty string. Also, if 0 ≤ k ≤ S.length,
kS = S[k, . . . S.length−1]. In the same way, S.lengthS is the
empty string. Edges in a transducer are of the form (p, a, b, q)
which means that there is an edge from the state p to the
state q reading the symbol a and writing the symbol b. A
question mark (?) on an edge transition (for example labeled
?/b) originating at state p stands for any input symbol that
does not appear as an input symbol on any other out-going
edge from p.
The transducer can be seen in three non-disjoint parts. The
first one corresponds to the processing of the prefix Pk−1 and
it is constructed in the same way that the automaton for string
matching is constructed and where each transduction outputs
the same symbol that it reads. When we have found the prefix
Pk in the text, we have two options: leave the symbol P[k]
unchanged or replace it by c. With the first option, we would
not expect to find all the pattern P because in the other case we
should have changed the symbol P[k] by c. With the second
option, we would expect to find all the pattern P in the text
because in the other case we would have make a mistake
changing the symbol P[k] in the text. Therefore, we need
to consider these two options in the transducer. In this way,
there will be two transductions from the same state: the first
one must read P[k] and output the same symbol P[k] and the
second one must read P[k] but output c. This leads us to the
second and third parts of the transducer, which correspond to
the two options that we have just described.
In our algorithm, lines from 2 to 9 basically describe the
same algorithm presented in [6] keeping track to the biggest
found prefix so far. This for loop constructs the first and the
second part of the transducer, where the input symbols are the
same for the output for every edge. The state number m =
P.length will be always a sink state, i.e. a state that has no
out-going edge which means that the processed input is not
going to be accepted by the transducer. The if sentence of the
line 10 is used to initialize the variable transNode which will
be the state where the edge with the transduction P[k]/c is
going to and where the third part of the transducer starts. The
loop for in the line 14 finishes the construction of the third part
of the transducer. For each character in P[k + 1, . . . , m − 2],
it adds an edge with the same input and out symbol and an
edge from each of these states to the sink state m with the
transduction ?/?. It means that if the pattern after the character
P[k] is not found, we have before made a mistake replacing
P[k] by c and this current input cannot be accepted. Lines
from 18 to 20 actually add the transition P[k]/c and complete
the third part of the transducer adding the edges from the last
state of this part. Finally, the for loop in the line 21 considers
the cases when the transduction P[k]/c must be realized from
the states of the second part of the transducer verifying that
this does not cause ambiguities.
LOCAL-EXTENSION(P, k, c)
1 m = P.length
2 for q = 0 to m − 1
3 for each character a ∈ Σ
4 j = min(m, q + 1)
5 repeat
6 j = j − 1
7 until Pj A Pq−1a
8 E = E ∪ {(q, a, a, j + 1)}
9 F = F ∪ {q}
10 if k = m − 1
11 transNode = 0
12 else
13 transNode = m + 1
14 for i = k + 1 to m − 2
15 node = m + i − k
16 E = E ∪ {(node, P[i], P[i], node + 1)}
17 E = E ∪ {(node, ?, ?, m)}
18 E = E ∪ {(k, P[k], c, transNode)}
19 E = E ∪ {(2m − k − 1, P[m − 1], P[m − 1], 0)}
20 E = E ∪ {(2m − k − 1, ?, ?, m)}
21 for i = k + 1 to m − 1
22 S = P[k . . i − 1]P[k . . m − 1]
23 if Pk−1 A Pi−1 and Sm−k−1 6@i−k S
24 E = E ∪ {(i, P[k], c, transNode)}
In figure 3 the result of the algorithm for the same contextual
rule of the transducer in the figure 1 is shown. In figure 4 it
is presented the local extension for the rule bbac → bbbc. The
first part is composed of the states 0, 1 and 2. The second one,
of the states 2, 3 and 4. And the third one, of the sates 2, 5
and 0. In this example, m = 4 is the sink state and the state
5 is the transState.
IV. EXPERIMENTS AND EVALUATION
For training and testing, it was used the data set for Spanish
provided in the CoNLL-2002. The training file was divided in
two parts: 85, 1% for the lexical tagger and 15, 9% for the
3
Figure 3: Local extension for the rule aa → ba.
Figure 4: Local extension for the rule bbac → bbbc.
Table I: Results for the data training sets.
data set precision recall Fβ=1
Spanish dev. 58, 9% 54, 5% 56, 6%
Spanish test 61, 8% 58, 3% 60, 0%
contextual tagger. Since some trigger words could belong to
some named entities, we decided to infer 100 rules in the
regular way, i.e. exploring among all possible rules, and then
infer about 15 rules which tried to replace those preposition
tags by BIO tags. These 115 rules produced a deterministic
transducer of 15.828 states.
The context length used in these rules was of size ±2.
We had made a previous experiment with size ±3, but we
noticed that it requires a huge amount of training time and the
transducer size was also big because these longer rules were
more complex and generated a more complex transducer when
their local extension transducers were composed. However,
the results with this configuration was similar to the ±2
configuration, therefore we adopted to work with the last one
configuration.
Table I presents the results of the system for the both data
sets provided int the CoNLL-2002. The final comparison used
the test data set. The development data set was given to the
participants for testing their models before the conference took
place. Table II describes the contextual tagger performance.
The first line presents the measures for the case when we only
used the lexical tagger. We can see that the support provided
for the contextual tagger is around 10%.
Table II: Performance of the contextual tagger.
precision recall Fβ=1
Before apply cont. rules 50, 3% 51, 4% 50, 8%
After apply cont. rules 61, 8% 58, 3% 60, 0%
V. DISCUSSION
It was shown a different approach for the NEE task where
a simple model totally based in automatons (or transducers) is
learned. The time complexity required to identify and classify
named entities in text is lineal. In fact, the text is examined
only two times: in the initialization stage using the data
structure trie and in the next stage, the application of the
transducer of the contextual rules. The results obtained are
comparable with other works presented in the CoNLL-2002
with significant more powerful techniques. For instance, in
[7] is introduced a system based on linear support vector
machines (SVMs) which had a precision of 56, 28% and
a recall 66, 51%, while in [8] it was used techniques like
transformation-based learning and decision trees obtaining a
precision of 60, 53% and a recall of 67, 29%. On overall,
both systems attained a F1 measure of 60, 97% and 63, 73%
respectively with the same data set.
Furthermore, as we have remarked before, the only assumption made in the system about the Spanish was the list of
trigger words. This allow us to easily apply the system a different languages without make substantial changes. Therefore,
we can still improve the proposed system as well as expand
it to more languages.
REFERENCES
[1] D. Jurafsky and J. H. Martin, Speech and Language Processing, 2nd ed.
Prentice Hall, 2008.
[2] C. Krstev, D. Vitas, I. Obradovic, and M. Utvi ´ c, “E-dictionaries ´
and finite-state automata for the recognition of named entities,”
in Proceedings of the 9th International Workshop on Finite State
Methods and Natural Language Processing, 2011. [Online]. Available:
http://www.aclweb.org/anthology/W11-4407
[3] M. Padro and L. Padr ´ o, “A named entity recognition system based on ´
a finite automata acquisition algorithm,” in Finite-State Methods and
Natural Language Processing. Prentice Hall, 2005. [Online]. Available:
http://www.cs.upc.edu/∼nlp/papers/padro05a.pdf
[4] E. Brill, “A simple rule-based part of speech tagger,” in Third Conference
on Applied Natural Language Processing, 1992, pp. 152–155. [Online].
Available: http://www.aclweb.org/anthology/A92-1021
[5] E. Roche and Y. Schabes, “Deterministic part-of-speech tagging with
finite state transducers,” no. TR94-07, May 1994. [Online]. Available:
http://www.merl.com/publications/TR94-07/
[6] T. H. Cormen, C. E. Leiserson, R. L. Rivest, and C. Stein, Introduction
to Algorithms, 3rd ed. The MIT Press, 2009.
[7] P. McNamee and J. Mayfield, “Entity extraction without languagespecific resources,” in Proceedings of CoNLL-2002. Taipei, Taiwan,
2002, pp. 183–186. [Online]. Available: http://www.clips.uantwerpen.be/
conll2002/pdf/18386mcn.pdf
[8] W. J. Black and A. Vasilakopoulos, “Language-independent named entity
classification by modified transformation-based learning and by decision
tree induction,” in Proceedings of CoNLL-2002. Taipei, Taiwan,
2002, pp. 159–162. [Online]. Available