Abstract:
Graph Convolutional Networks (GCNs) have received considerable attention in the field of artificial machine intelligence (AMI) and natural language processing research because they can build more sophisticated accompanying graph structures than traditional neural networks for feature engineering. Graph is used as feature in neural network because it is easy to find relations among nodes. In text classification applications, a GCN can create complex and rich relation-based adjacent matrix graphs as features to be trained. The existing methods, on the other hand, only generated adjacent matrix graphs in GCN at the word-document and word-word levels as features. In this paper, we propose a document-relational GCN to achieve superior accuracy in text classification by adding cumulative term frequency-inverse document frequency (TF-IDF) document-document relations as features. The performance of the proposed method is evaluated using five popular benchmark databases. In addition, different hidden nodes and proportions of document-document features are tested to achieve an advantageous outcome.

A document-relational GCN is to add cumulative term frequency-inverse document frequency (TF-IDF) document-document relations as features. In addition, different hidden nodes and proportions of document-document features are tested to achieve an advantageous outcome.
A document-relational GCN is to add cumulative term frequency-inverse document frequency (TF-IDF) document-document relations as features. In addition, different hidden n...Show More
Published in: IEEE Access ( Volume: 10)
Page(s): 123205 - 123211
Date of Publication: 14 November 2022 
Electronic ISSN: 2169-3536
DOI: 10.1109/ACCESS.2022.3221820
Publisher: IEEE
CCBY - IEEE is not the copyright holder of this material. Please follow the instructions via https://creativecommons.org/licenses/by/4.0/ to obtain full-text articles and stipulations in the API documentation.
SECTION I.Introduction
Text classification is one of the most important tasks in deep learning problems. It recognises and classifies documents for different applications, such as sentiment classification [1], semantic segmentation [2], rule text based recommendation [3] and news classification [4]. Many researchers have attempted to solve text classification problems in different text ranges, including at document [5] and sentence levels [6].

To achieve better classification performance, significant efforts have been made to accelerate the process of model development from machine learning methods [7], [8] to deep learning methods [2], [9], [10]. And the development of deep learning computing efficiency [11], [12] techniques have been assisting running time of deep learning much faster. Compared to other methods for text classification, deep learning methods have achieved better effective and efficient performance owing to the rapid development of computer hardware and deep learning research.

Topic classification is an important text classification application that uses machine learning [13], [14], [15], and deep learning [16], [17], [18] methods. For example, in road construction documents, a text classification program needs to be developed to categorise the regulatory compliance language. Document inspection for road construction regulatory compliance begins with text classification, as described in [19] and [20]. As different forms of code compliance documents have varied roles in terms of topics, topic classification is an appropriate solution for code compliance text classification.

News classification is one of the most popular and widely used applications in the field of topic classifications. Every minute, there are so many news releases that individuals cannot keep up with them. It is possible to classify news in a manner that allows humans to comprehend it more quickly and correctly. On Facebook and Twitter, for example, a plethora of news has been disseminated. The news should be classified into their own categories [21]. As a result, people and social media firms should be responsible to structure news data. To comprehend data using the Internet structural approach, Trieu et al. [22] developed a word vector technique for news classification based on Twitter data. Furthermore, false news accounts for a significant proportion of online news releases. Recently, fake news classification has improved, which can help consumers detect false news [23], [24], [25]. Furthermore, news classification can assist healthcare companies in extracting relevant data from social media news [26]. Therefore, news classification is worth investigating to improve its performance effectively and efficiently.

In recent years, graph networks have received considerable interest in deep learning research [27], and the use of graph networks for text classification has shown excellent performance [28], [29], [30]. Because many applications may build huge amount of data in a structured fashion, graphs have the benefit of storing rich relations between nodes [31]. Then, this type of structural data can be used as features in a deep learning model to obtain good results for a realistic relation. The development of deep learning networks has been assisted the development of GCN, such as Convolutional Neural Network (CNN) [32], [33]. To obtain better citation classification performance in an effective and efficient manner, Kipf and Welling [34] developed a GCN model that can build relations of objects throughout the entire dataset. The GCN model was then used to classify documents by Yao et al. [35], who achieved improved performance in five distinct benchmark datasets, including a news dataset. However, in their GCN model, they only generated the feature graph using document-word relations. Tang et al. [36] constructed a text classification integration model based on a GCN and achieved competitive results. A document-document link was added into feature graph to seek a better result for text classification.

Till now, there have been a few techniques that can be used to enhance graph features. Meta-path is a type of text classification characteristic that can be used to create document relations. Wang et al. [37] utilised a meta-path approach to build linkages between documents, and a machine learning classifier produced superior classification results in two topic-based datasets. The meta-path approach was used by Ding et al. [38] and was applied to enrich the knowledge graph for the entire dataset. In summary, it can be concluded that the link relation between the generated documents in the dataset has better performance on topic sensitive documents. Therefore, text classification effect can be improved from the perspective of feature engineering to enhance the knowledge network in the entire dataset. In conclusion, the link relations between the documents produced in the dataset performs better on the topic sensitive documents to enhance the text classification.

Working on feature engineering is an effective way to enhance the text classification performance. Data are the basis of feature engineering to achieve good performance, and almost all models require data to drive [39], [40]. There are two methods of feature engineering which are searching relations inside a dataset, such as graph networks [27], [28], [29], [30] and increasing size of one dataset, such as data augmentation [41], [42]. Zhang et al. [43] found an effective way to perform data augmentation which simply replaced words or phrases in the dataset by their synonym ones.

We propose to add document-document link weights to the data graph to improve text classification performance, inspired by the notion of adding visual information to features through meta-paths in terms of feature engineering. Also, we compare our method to text graph networks and synonym data augmentation. The contributions of this study are as follows:

We generate a cumulative term frequency-inverse document frequency (TF-IDF) value for document-document relations to achieve higher text classification accuracy.

We tuned an appropriate hyperparameter value in the document-relational GCN model to obtain the best text classification performance.

The remaining parts of this paper are organised as follows. We present previous GCN models in Section 2. The proposed cumulative TF-IDF document-relational GCN model is established in Section 3. Section 4 presents the experimental results and their analysis. Finally, the conclusions of this paper and the prospects for future work are presented in Section 5.
SECTION II.Related Work
In this section, we briefly describe previous works on document classification for both graph generation and GCN model construction.

A. Graph Generation
Yao et al. [35] constructed a large heterogeneous graph for an entire dataset. The total number of nodes in a corpus was equal to the number of documents and unique words. Thus, the dimension of the graph adjacent matrix is (documents+uniquewords)×(documents+uniquewords) . Furthermore, in the graph, there are two types of edge relations which are document-word and word-word. TF-IDF determines the values of document-word edges:
TFi,j=IDFi=TFIDFi,j=Ni,j∑k∈jNk,jlog|D||j:ti∈dj|+1TFi,j×IDFi(1)
View SourceRight-click on figure for MathML and additional features.where TFi,j is frequency of the term (word) i in the document j , and IDFi is the logarithmic number of the total documents divided by the number of the documents that contain the word i .

Word-word relations are represented by another sort of edges. To deal with these relations, traditional approaches are to count word occurrences in a confined context region, which is known as word co-occurrence statistics [44], [45], [46]. The word-word link weight value in the constructed graph is calculated using the point-wise mutual information (PMI) method, which is based on the word co-occurrence concept. To track and calculate single and double words co-occurrence frequency, a convolutional window (a fixed length with 20 words) is used. The PMI may be computed using the following formula:
PMI(i,j)=p(i,j)=p(i)=logp(i,j)p(i)p(j)#W(i,j)#W#W(i)#W(2)
View SourceRight-click on figure for MathML and additional features.where W(i) counts the number of slide windows that contain word i , W(i,j) counts the number of slide windows that contain words i and j , and #W is the total number of windows sliding past all the documents. A greater PMI value implies a higher correlation between the two words, whereas a lower value suggests a lower correlation between two words. In the graph, only positive PMI values are utilised to indicate the edge of the word-word relation. The negative PMI value is set to zero, indicating that there is no edge in the node pair.

B. GCN Model
The document graph in the previous subsection was used as a feature in the GCN model for further training and testing. Kipf and Welling [34] developed a GCN model expressed by a multilayer neural network that directly operates on graphs as features. Let G=(N,E) denote the document graph, where N and E are the nodes and edges, respectively. The number of nodes is the sum of the number of documents and unique vocabulary terms. Adjacent matrix A is used to represent the graph. Then, the adjacent matrix A must be normalised as A~=D−0.5AD−0.5 , where D is the degree matrix of A and Dii=∑jAij . There is a feature matrix X followed by a normalised adjacent matrix A~ . In the GCN model for text classification, the feature matrix is usually set to be an identity matrix as X=I , so X is not shown in the future equations. The normalised adjacent matrix A~ is a feature of the model. The forward propagation equations for the text classification version are as follows:
L1=Lj+1=A~=Dii=r(x)=r(A~W0)r(A~LjWj)D−1/2AD−1/2∑jAijmax(0,x)(3)
View SourceRight-click on figure for MathML and additional features.where L is the hidden or result layer matrix, j is the layer number, A~ is the normalised adjacent matrix of the graph, and r(x) is an activation function of the neural network, called ReLU.

SECTION III.Proposed Method
In this section, we develop cumulative TF-IDF edges to connect document-document nodes, which differs from the graph that simply uses word-document relations. We generate document relations by adding the TF-IDF values multiplication of the same words.
A′ij=log(A(rowi)⋅A(columnj))(4)
View SourceRight-click on figure for MathML and additional features.where i,j are document indices, A(rowi) is the row vector for document i , and A(columnj) is the column vector for document j . The multiplication of document vectors i and j shows that the TF-IDF values of two documents with the same vocabulary are multiplied and added together. Then we only save the edges that are greater than or equal to 3 because only a large value of document-document relation has a positive effect on text classification performance. Other edge relations of the graph, such as word-word and document-word relations, are identical to those in [35]. Thus, the total adjacent matrix can be defined as follows:
A′ij=⎧⎩⎨⎪⎪⎪⎪⎪⎪⎪⎪⎪⎪log(A(rowi)⋅A(columnj))PMI(i,j)TF−IDFi,j10 i,j are documentsi,j are wordsi is document,j is wordi=jotherwise
View SourceRight-click on figure for MathML and additional features.Document-document, word-document, and word-word relational values comprise adjacency matrix A′ij . The document-document relation is obtained from the equation (4). The word-document relation is the TF-IDF value from the equation (1). The word-word relation is PMI value from the equation (2). The adjacent matrix A′ij has a square shape. The document and vocabulary together constitute the size of the adjacency matrix. The details of how to construct the neighbouring matrix of the graph are shown in Pseudo Code 1.

Pseudo Code 1 - Build an Adjacent Matrix of the Graph

Pseudo Code 1
Build an Adjacent Matrix of the Graph

Show All

The adjacent matrix of graph A′ij is then sent to the GCN model in the next step. A previous study ([34], [35], [47]) shows that a GCN model with two layers performs better than that with one layer, whereas a GCN model with more than two layers does not perform better than that with two layers. Thus, we use a two-layer GCN model to train and test our text classification model in this study. Let
Z=softmax(xi)=A~=Dii=softmax(A~⋅r(A~W0)W1),exp(xi)∑iexp(xi)D−1/2A′D−1/2∑jA′ij(5)
View SourceRight-click on figure for MathML and additional features.where A~ is the same as that in (3), W0 and W1 are two weight matrices to learn, and softmax(⋅) is a probability normalisation function that can convert the result vector elements into a probability distribution. For each vector, the number of elements is the number of document classes that must be classified. Thus, the total of all elements in a vector after the softmax computation is 1, and the highest value position represents the text classification class. Then, the cross-entropy loss function is applied to calculate the error distance between the model forward propagation result Z and the target label:
Loss=−∑d∈Yd∑c=1CYdclogZdc(6)
View SourceRight-click on figure for MathML and additional features.where Yd is the label document index, and C is the number of document class dimensions of the outcome feature. Ydc is a vector label and Zdc is a vector result. They represent the target label and predicted result of a document, respectively.

The loss function has two weights that must be learned. The GCN model does forward propagation and calculates the loss function each time. The derivatives of two weights (W0 and W1 ) are then determined. To reduce loss value, the W0 and W1 are updated in the opposite direction as the derivatives. The optimal weights (W0 and W1 ) are discovered, and it can now approach the best document classification prediction using the present model and inputs.

SECTION IV.Experiment
In this section, for text classification accuracy, we compare our document-document updated GCN to the original text GCN and augmented data with text GCN. Synonym data augmentation [43] is used to enlarge the five datasets which is double the size of each dataset. Furthermore, we verify the accuracy of our model by filtering alternative maximum values of document-document graph weights in adjacent matrix of the graph. In the updated document GCN model, we evaluate the performance for different numbers of hidden layer dimensions.

A. Datasets
In the text classification field, we consider five benchmark datasets.

20ng. Dataset of 20 newsgroup records and 20 different types of news. There are a total of 18846 documents and 42757 vocabulary items. 11314 documents are used for training and 7532 for testing.

R8 and R52. Two subsets of Reuters data. There are 8 and 20 classes in each. R8 is equipped with 5485 train documents, 2189 test documents, and 7688 vocabulary items. R52 has 6532 train documents, 2568 test documents, and 8892 vocabulary items in its database.

Ohsumed [48]. It comes from the “MEDLINE10” medical database, which contains titles and abstracts from 270 medical journal articles published between 1987 and 1992. Diseases are divided into 23 categories.

MR. Dataset of movie reviews with only two sentiment classes. There are 10662 documents in total, with half of them being positive reviews and the other half being negative reviews.

The details of the datasets are summarised in Table 1. The number of training nodes is the sum of the number of true training documents and the number of valid training documents. The training documents are divided into two categories: actual training and validation. 90% of training documents are for actual training, while 10% are for validation.

TABLE 1 Datasets Summary
Table 1- 
Datasets Summary

B. Experiment Parameters
The PyTorch library is one of the most widely used deep learning frameworks. Both GCN models (document-relational GCN and original text GCN model) are run on the PyTorch deep learning framework. In the training model, the weights for both models are updated using the Adam optimiser [49]. As previously stated, the drop out rate is 0.5, and the GCN model has two layers.

C. Hidden Layers
Yao et al. [35] chose 200 hidden layer dimensions of the GCN model in their experiment. According to our research, it is difficult to obtain good test performance with as few as 200 hidden layer dimensions, according to our research. Consequently, a number of hidden layer dimensions are investigated, as shown in Fig. 1.

FIGURE 1. - Relations Between the Number of Hidden Layers and Test Accuracy. When we used 1200 and 1500 hidden layers in our document-relational GCN model, we found that we could achieve better test accuracy.

FIGURE 1.
Relations Between the Number of Hidden Layers and Test Accuracy. When we used 1200 and 1500 hidden layers in our document-relational GCN model, we found that we could achieve better test accuracy.

Show All

D. Experiment Results
The test accuracy comparison for our document-relational GCN and the original model without document-document relation edges is presented in Table 2. Our new model is even more accurate than the original GCN model, which performed admirably in the previous experiment [35]. In four out of five datasets, our document-relational GCN model improves 0.2%-1% accuracy to text GCN. For the 20ng dataset, the model exhibits a noticeable performance improvement (1%) in terms of test accuracy compared to other datasets. Because of the new document-document relation in the graph, it performs better for the 20ng dataset than the others. The synonym augmented GCN performs better in the R52 and Ohsumed datasets. TF-IDF stores important words that can represent document topics, and contains information about document-document relations. It also increases the weight of topic words in documents on the same subject. Because the same news category belongs to the same topic, 20ng is both a news and topic classification dataset. In a hyperparameter experiment with hidden layers, overall, there is an increasing tendency of accuracy with an increasing number of hidden layers in our document-relational GCN model. The most accurate results are obtained with the number of hidden layers ranging from 1200 to 1500 in the four datasets. We can hardly obtain good results in both test accuracy and expected hidden layers for the Ohsumed dataset because using words link to stand for document is not sensitive to medical abstract documents. Overall, the cumulative TF-IDF document-document relation in our document-relational GCN can achieve much higher test accuracy in most datasets related to topic document classification, and a specific figure of the hidden layer hyperparameter can be used for model parameter tuning.

TABLE 2 Comparison Test Accuracy Among Document-Relational GCN, Text GCN and Synonym Augmented Text GCN
Table 2- 
Comparison Test Accuracy Among Document-Relational GCN, Text GCN and Synonym Augmented Text GCN

SECTION V.Conclusion
We have introduced a new document-relational GCN in this study, which provides a cumulative TF-IDF document-document relation for text classification. By testing its performance on five benchmark datasets, we discovered that with document-document relation edges in the adjacent matrix, the document-relational GCN model can improve the overall accuracy comparing to the original text GCN and the synonym augmented GCN. Synonym augmented GCN has higher accuracy in the medical dataset. In addition, we experiment with different numbers of hidden layers to determine the optimal parameters of the model. We can see that excessive hidden layers do not help the original text GCN and data-augmented GCN have higher accuracy. However, in the document-relational GCN model, a large number of hidden layers have a better ability to store text and graph features, which helps text classification. Our future research plan is to determine the best percentage of document-document edges to use when filtering useful or important document relation features. We will also use a more sophisticated algorithm to calculate document-document relations, as well as filtering document-word or word-word relations to simplify the graph.