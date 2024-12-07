-- use merge join for primary key joins
explain select * from t1 join t2 on a = c;

/*
Join { type: inner, on: (c = a), cost: 0, rows: 0 }
├── Scan { table: t1, list: [ a, b ], filter: true, cost: 0, rows: 0 }
└── Scan { table: t2, list: [ c, d ], filter: true, cost: 0, rows: 0 }
*/

-- use storage order by instead of sorting by primary key
explain select * from t1 order by a;

/*
Scan { table: t1, list: [ a, b ], filter: true, cost: 0, rows: 0 }
*/

-- use storage filter for primary key
explain select * from t1 where a = 1;

/*
Scan { table: t1, list: [ a, b ], filter: (a = 1), cost: 10, rows: 5 }
*/

-- use storage filter for a combination of primary key and other keys
explain select * from t1 where a > 1 and a < 3 and b > 1;

/*
Filter { cond: (b > 1), cost: 16.05, rows: 2.5 }
└── Scan { table: t1, list: [ a, b ], filter: ((3 > a) and (a > 1)), cost: 10, rows: 5 }
*/

-- use storage filter for a combination of primary key (always false) and other keys
explain select * from t1 where a > 1 and a < 0 and b > 1;

/*
Scan { table: t1, list: [ a, b ], filter: false, cost: 10, rows: 5 }
*/

-- use storage filter for a combination of primary key (could be eliminated) and other keys
explain select * from t1 where a > 1 and a > 3 and b > 1;

/*
Filter { cond: ((a > 3) and (b > 1)), cost: 15.1, rows: 1.25 }
└── Scan { table: t1, list: [ a, b ], filter: (a > 1), cost: 10, rows: 5 }
*/

