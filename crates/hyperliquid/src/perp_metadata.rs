pub const PERP_META: &'static str = r#"[
    {
            "universe": [
                {
                    "szDecimals": 5,
                    "name": "BTC",
                    "maxLeverage": 40,
                    "marginTableId": 56
                },
                {
                    "szDecimals": 4,
                    "name": "ETH",
                    "maxLeverage": 25,
                    "marginTableId": 55
                },
                {
                    "szDecimals": 2,
                    "name": "ATOM",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "MATIC",
                    "maxLeverage": 20,
                    "marginTableId": 20,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "DYDX",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 2,
                    "name": "SOL",
                    "maxLeverage": 20,
                    "marginTableId": 54
                },
                {
                    "szDecimals": 2,
                    "name": "AVAX",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 3,
                    "name": "BNB",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 1,
                    "name": "APE",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "OP",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 2,
                    "name": "LTC",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "ARB",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 0,
                    "name": "DOGE",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "INJ",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 1,
                    "name": "SUI",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 0,
                    "name": "kPEPE",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "CRV",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "LDO",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 1,
                    "name": "LINK",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "STX",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "RNDR",
                    "maxLeverage": 20,
                    "marginTableId": 20,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "CFX",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "FTM",
                    "maxLeverage": 10,
                    "marginTableId": 10,
                    "isDelisted": true
                },
                {
                    "szDecimals": 2,
                    "name": "GMX",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "SNX",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "XRP",
                    "maxLeverage": 20,
                    "marginTableId": 53
                },
                {
                    "szDecimals": 3,
                    "name": "BCH",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 2,
                    "name": "APT",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 2,
                    "name": "AAVE",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 2,
                    "name": "COMP",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 4,
                    "name": "MKR",
                    "maxLeverage": 10,
                    "marginTableId": 51,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "WLD",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "FXS",
                    "maxLeverage": 5,
                    "marginTableId": 5,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "HPOS",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 0,
                    "name": "RLB",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 3,
                    "name": "UNIBOT",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 0,
                    "name": "YGG",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "TRX",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 0,
                    "name": "kSHIB",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 1,
                    "name": "UNI",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 0,
                    "name": "SEI",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 1,
                    "name": "RUNE",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "OX",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 1,
                    "name": "FRIEND",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 0,
                    "name": "SHIA",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 1,
                    "name": "CYBER",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "ZRO",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "BLZ",
                    "maxLeverage": 5,
                    "marginTableId": 5,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "DOT",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 1,
                    "name": "BANANA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 2,
                    "name": "TRB",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "FTT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "LOOM",
                    "maxLeverage": 10,
                    "marginTableId": 10,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "OGN",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "RDNT",
                    "maxLeverage": 5,
                    "marginTableId": 5,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "ARK",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "BNT",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "CANTO",
                    "maxLeverage": 5,
                    "marginTableId": 5,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "REQ",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "BIGTIME",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "KAS",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ORBS",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "BLUR",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "TIA",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 2,
                    "name": "BSV",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ADA",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "TON",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 0,
                    "name": "MINA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "POLYX",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "GAS",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "PENDLE",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "STG",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "FET",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "STRAX",
                    "maxLeverage": 10,
                    "marginTableId": 10,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "NEAR",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 0,
                    "name": "MEME",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 2,
                    "name": "ORDI",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "BADGER",
                    "maxLeverage": 5,
                    "marginTableId": 5,
                    "isDelisted": true
                },
                {
                    "szDecimals": 2,
                    "name": "NEO",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 2,
                    "name": "ZEN",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "FIL",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "PYTH",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "SUSHI",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 2,
                    "name": "ILV",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "IMX",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "kBONK",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 0,
                    "name": "GMT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "SUPER",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "USTC",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "NFTI",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 0,
                    "name": "JUP",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 0,
                    "name": "kLUNC",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "RSR",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "GALA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "JTO",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "NTRN",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 2,
                    "name": "ACE",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "MAV",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "WIF",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "CAKE",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "PEOPLE",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 2,
                    "name": "ENS",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 2,
                    "name": "ETC",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "XAI",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "MANTA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "UMA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ONDO",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 0,
                    "name": "ALT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "ZETA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "DYM",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "MAVIA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "W",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 5,
                    "name": "PANDORA",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "isDelisted": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 1,
                    "name": "STRK",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "PIXEL",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "AI",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 3,
                    "name": "TAO",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 2,
                    "name": "AR",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "MYRO",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "kFLOKI",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "BOME",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "ETHFI",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "ENA",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "MNT",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "TNSR",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "SAGA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "MERL",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "HBAR",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "POPCAT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 2,
                    "name": "OMNI",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 2,
                    "name": "EIGEN",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "REZ",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "NOT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "TURBO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "BRETT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "IO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ZK",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "BLAST",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "LISTA",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "MEW",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "RENDER",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "kDOGS",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "POL",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "CATI",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "CELO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "HMSTR",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "SCR",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "NEIROETH",
                    "maxLeverage": 5,
                    "marginTableId": 5,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "kNEIRO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "GOAT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "MOODENG",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "GRASS",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "PURR",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "PNUT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "XLM",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "CHILLGUY",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "SAND",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "IOTA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ALGO",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 2,
                    "name": "HYPE",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "ME",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "MOVE",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "VIRTUAL",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "PENGU",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "USUAL",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "FARTCOIN",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "AI16Z",
                    "maxLeverage": 5,
                    "marginTableId": 5,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "AIXBT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ZEREBRO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "BIO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "GRIFFAIN",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "SPX",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "S",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "MORPHO",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "TRUMP",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 1,
                    "name": "MELANIA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ANIME",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "VINE",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 2,
                    "name": "VVV",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "JELLY",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 1,
                    "name": "BERA",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "TST",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "LAYER",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "IP",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "OM",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "KAITO",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "NIL",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 3,
                    "name": "PAXG",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 0,
                    "name": "PROMPT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "BABY",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "WCT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "HYPER",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ZORA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "INIT",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "DOOD",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "LAUNCHCOIN",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "isDelisted": true
                },
                {
                    "szDecimals": 0,
                    "name": "NXPC",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "SOPH",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "RESOLV",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "SYRUP",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "PUMP",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 0,
                    "name": "PROVE",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "YZY",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "XPL",
                    "maxLeverage": 10,
                    "marginTableId": 51
                },
                {
                    "szDecimals": 0,
                    "name": "WLFI",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "LINEA",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "SKY",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "ASTER",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "AVNT",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "STBL",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "0G",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "HEMI",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "APEX",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "2Z",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 2,
                    "name": "ZEC",
                    "maxLeverage": 10,
                    "marginTableId": 52
                },
                {
                    "szDecimals": 0,
                    "name": "MON",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "MET",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "MEGA",
                    "maxLeverage": 3,
                    "marginTableId": 3,
                    "onlyIsolated": true,
                    "marginMode": "strictIsolated"
                },
                {
                    "szDecimals": 0,
                    "name": "CC",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 1,
                    "name": "ICP",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 0,
                    "name": "AERO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "STABLE",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "FOGO",
                    "maxLeverage": 3,
                    "marginTableId": 3
                },
                {
                    "szDecimals": 0,
                    "name": "LIT",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 3,
                    "name": "XMR",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 1,
                    "name": "AXS",
                    "maxLeverage": 5,
                    "marginTableId": 5
                },
                {
                    "szDecimals": 2,
                    "name": "DASH",
                    "maxLeverage": 5,
                    "marginTableId": 5
                }
            ],
            "marginTables": [
                [
                    50,
                    {
                        "description": "",
                        "marginTiers": [
                            {
                                "lowerBound": "0.0",
                                "maxLeverage": 50
                            }
                        ]
                    }
                ],
                [
                    51,
                    {
                        "description": "tiered 10x",
                        "marginTiers": [
                            {
                                "lowerBound": "0.0",
                                "maxLeverage": 10
                            },
                            {
                                "lowerBound": "3000000.0",
                                "maxLeverage": 5
                            }
                        ]
                    }
                ],
                [
                    52,
                    {
                        "description": "tiered 10x (2)",
                        "marginTiers": [
                            {
                                "lowerBound": "0.0",
                                "maxLeverage": 10
                            },
                            {
                                "lowerBound": "20000000.0",
                                "maxLeverage": 5
                            }
                        ]
                    }
                ],
                [
                    53,
                    {
                        "description": "tiered 20x",
                        "marginTiers": [
                            {
                                "lowerBound": "0.0",
                                "maxLeverage": 20
                            },
                            {
                                "lowerBound": "40000000.0",
                                "maxLeverage": 10
                            }
                        ]
                    }
                ],
                [
                    54,
                    {
                        "description": "tiered 20x (2)",
                        "marginTiers": [
                            {
                                "lowerBound": "0.0",
                                "maxLeverage": 20
                            },
                            {
                                "lowerBound": "70000000.0",
                                "maxLeverage": 10
                            }
                        ]
                    }
                ],
                [
                    55,
                    {
                        "description": "tiered 25x",
                        "marginTiers": [
                            {
                                "lowerBound": "0.0",
                                "maxLeverage": 25
                            },
                            {
                                "lowerBound": "100000000.0",
                                "maxLeverage": 15
                            }
                        ]
                    }
                ],
                [
                    56,
                    {
                        "description": "tiered 40x",
                        "marginTiers": [
                            {
                                "lowerBound": "0.0",
                                "maxLeverage": 40
                            },
                            {
                                "lowerBound": "150000000.0",
                                "maxLeverage": 20
                            }
                        ]
                    }
                ]
            ],
            "collateralToken": 0
        },
        [
            {
                "funding": "-0.000025387",
                "openInterest": "33193.39724",
                "prevDayPx": "92661.0",
                "dayNtlVlm": "2621135493.3779191971",
                "premium": "-0.0006674644",
                "oraclePx": "91091.0",
                "markPx": "91029.0",
                "midPx": "91026.5",
                "impactPxs": [
                    "91026.0",
                    "91030.2"
                ],
                "dayBaseVlm": "28387.92009"
            },
            {
                "funding": "-0.0000060428",
                "openInterest": "965550.1860000001",
                "prevDayPx": "3203.1",
                "dayNtlVlm": "1558717977.2503299713",
                "premium": "-0.0004478424",
                "oraclePx": "3126.1",
                "markPx": "3124.4",
                "midPx": "3124.65",
                "impactPxs": [
                    "3124.6",
                    "3124.7"
                ],
                "dayBaseVlm": "488517.4130999998"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1712372.3799999999",
                "prevDayPx": "2.3721",
                "dayNtlVlm": "637980.1485839997",
                "premium": "0.0",
                "oraclePx": "2.4275",
                "markPx": "2.4261",
                "midPx": "2.4263",
                "impactPxs": [
                    "2.4233",
                    "2.4281"
                ],
                "dayBaseVlm": "261805.81"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.37621",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.3754",
                "markPx": "0.37621",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "15413242.4000000004",
                "prevDayPx": "0.17704",
                "dayNtlVlm": "206993.9548529999",
                "premium": "0.0",
                "oraclePx": "0.17275",
                "markPx": "0.17271",
                "midPx": "0.17266",
                "impactPxs": [
                    "0.1723",
                    "0.17289"
                ],
                "dayBaseVlm": "1168658.1999999995"
            },
            {
                "funding": "-0.000011943",
                "openInterest": "4733664.5599999996",
                "prevDayPx": "133.32",
                "dayNtlVlm": "313130063.1633000374",
                "premium": "-0.0004088625",
                "oraclePx": "131.34",
                "markPx": "131.27",
                "midPx": "131.275",
                "impactPxs": [
                    "131.27",
                    "131.2863"
                ],
                "dayBaseVlm": "2347917.1000000006"
            },
            {
                "funding": "-0.000002598",
                "openInterest": "1173423.9000000001",
                "prevDayPx": "12.625",
                "dayNtlVlm": "3431948.11901",
                "premium": "-0.0003646453",
                "oraclePx": "12.615",
                "markPx": "12.608",
                "midPx": "12.6075",
                "impactPxs": [
                    "12.607",
                    "12.6104"
                ],
                "dayBaseVlm": "269920.7"
            },
            {
                "funding": "0.0000125",
                "openInterest": "53580.496",
                "prevDayPx": "924.86",
                "dayNtlVlm": "8184109.9494299982",
                "premium": "-0.0000327097",
                "oraclePx": "917.16",
                "markPx": "916.86",
                "midPx": "917.065",
                "impactPxs": [
                    "917.006",
                    "917.13"
                ],
                "dayBaseVlm": "8846.96"
            },
            {
                "funding": "0.0000055477",
                "openInterest": "4639386.1999999993",
                "prevDayPx": "0.19798",
                "dayNtlVlm": "310155.312977",
                "premium": "0.0",
                "oraclePx": "0.19625",
                "markPx": "0.196",
                "midPx": "0.1961",
                "impactPxs": [
                    "0.19593",
                    "0.19632"
                ],
                "dayBaseVlm": "1555811.8000000003"
            },
            {
                "funding": "-0.0000061481",
                "openInterest": "14372643.0",
                "prevDayPx": "0.31041",
                "dayNtlVlm": "730099.7298469999",
                "premium": "0.0",
                "oraclePx": "0.31215",
                "markPx": "0.31195",
                "midPx": "0.312",
                "impactPxs": [
                    "0.3119",
                    "0.31224"
                ],
                "dayBaseVlm": "2320895.100000001"
            },
            {
                "funding": "0.0000034769",
                "openInterest": "443502.62",
                "prevDayPx": "70.21",
                "dayNtlVlm": "10993290.6859999988",
                "premium": "-0.0001111032",
                "oraclePx": "70.205",
                "markPx": "70.168",
                "midPx": "70.1775",
                "impactPxs": [
                    "70.1721",
                    "70.1972"
                ],
                "dayBaseVlm": "155940.93"
            },
            {
                "funding": "0.0000074319",
                "openInterest": "19005869.3999999985",
                "prevDayPx": "0.19294",
                "dayNtlVlm": "1978786.0578940012",
                "premium": "0.0",
                "oraclePx": "0.19086",
                "markPx": "0.19073",
                "midPx": "0.19082",
                "impactPxs": [
                    "0.19075",
                    "0.19087"
                ],
                "dayBaseVlm": "10287503.8000000007"
            },
            {
                "funding": "0.0000125",
                "openInterest": "452971840.0",
                "prevDayPx": "0.12698",
                "dayNtlVlm": "19965096.2718799971",
                "premium": "-0.0000786349",
                "oraclePx": "0.12717",
                "markPx": "0.12711",
                "midPx": "0.12715",
                "impactPxs": [
                    "0.12714",
                    "0.12716"
                ],
                "dayBaseVlm": "155048329.0"
            },
            {
                "funding": "0.0000042798",
                "openInterest": "389609.2",
                "prevDayPx": "4.675",
                "dayNtlVlm": "516017.72274",
                "premium": "-0.0002051174",
                "oraclePx": "4.729",
                "markPx": "4.7265",
                "midPx": "4.72625",
                "impactPxs": [
                    "4.72374",
                    "4.72803"
                ],
                "dayBaseVlm": "108413.1"
            },
            {
                "funding": "-0.0000171094",
                "openInterest": "17665954.6000000015",
                "prevDayPx": "1.5635",
                "dayNtlVlm": "20748556.1431099996",
                "premium": "-0.0004530451",
                "oraclePx": "1.5451",
                "markPx": "1.544",
                "midPx": "1.54435",
                "impactPxs": [
                    "1.54423",
                    "1.5444"
                ],
                "dayBaseVlm": "13292114.5"
            },
            {
                "funding": "-0.0000186907",
                "openInterest": "3498081254.0",
                "prevDayPx": "0.005219",
                "dayNtlVlm": "11286780.3401060011",
                "premium": "-0.0005798222",
                "oraclePx": "0.005174",
                "markPx": "0.00517",
                "midPx": "0.00517",
                "impactPxs": [
                    "0.005168",
                    "0.005171"
                ],
                "dayBaseVlm": "2157981800.0"
            },
            {
                "funding": "0.0000106493",
                "openInterest": "20740321.0",
                "prevDayPx": "0.38937",
                "dayNtlVlm": "3790471.2371269995",
                "premium": "0.0",
                "oraclePx": "0.3906",
                "markPx": "0.39",
                "midPx": "0.3903",
                "impactPxs": [
                    "0.39018",
                    "0.39067"
                ],
                "dayBaseVlm": "9733490.9000000004"
            },
            {
                "funding": "-0.0000008567",
                "openInterest": "9561964.4000000004",
                "prevDayPx": "0.55301",
                "dayNtlVlm": "2309333.3131970004",
                "premium": "-0.0000733272",
                "oraclePx": "0.5455",
                "markPx": "0.54517",
                "midPx": "0.54536",
                "impactPxs": [
                    "0.54512",
                    "0.54546"
                ],
                "dayBaseVlm": "4178467.3000000007"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1974939.2",
                "prevDayPx": "12.75",
                "dayNtlVlm": "5860881.9777000034",
                "premium": "-0.0001579155",
                "oraclePx": "12.665",
                "markPx": "12.66",
                "midPx": "12.6625",
                "impactPxs": [
                    "12.6598",
                    "12.663"
                ],
                "dayBaseVlm": "456084.1000000002"
            },
            {
                "funding": "0.0000125",
                "openInterest": "3458749.0",
                "prevDayPx": "0.32606",
                "dayNtlVlm": "183041.3420290001",
                "premium": "-0.0002839565",
                "oraclePx": "0.31695",
                "markPx": "0.31664",
                "midPx": "0.31671",
                "impactPxs": [
                    "0.31655",
                    "0.31686"
                ],
                "dayBaseVlm": "565097.6000000001"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "6.8946",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "7.03",
                "markPx": "6.8952",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000030749",
                "openInterest": "9659018.0",
                "prevDayPx": "0.072449",
                "dayNtlVlm": "178969.924373",
                "premium": "0.0",
                "oraclePx": "0.0724",
                "markPx": "0.072365",
                "midPx": "0.072375",
                "impactPxs": [
                    "0.072335",
                    "0.072402"
                ],
                "dayBaseVlm": "2437144.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.73",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.712",
                "markPx": "0.71688",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0000066032",
                "openInterest": "48813.9",
                "prevDayPx": "7.299",
                "dayNtlVlm": "137542.171844",
                "premium": "0.0",
                "oraclePx": "7.149",
                "markPx": "7.1413",
                "midPx": "7.1441",
                "impactPxs": [
                    "7.1391",
                    "7.1491"
                ],
                "dayBaseVlm": "18877.74"
            },
            {
                "funding": "-0.0000368539",
                "openInterest": "1379537.2",
                "prevDayPx": "0.43982",
                "dayNtlVlm": "278670.4364709999",
                "premium": "-0.0004639295",
                "oraclePx": "0.4311",
                "markPx": "0.43035",
                "midPx": "0.4303",
                "impactPxs": [
                    "0.42996",
                    "0.4309"
                ],
                "dayBaseVlm": "636136.8"
            },
            {
                "funding": "0.0000075055",
                "openInterest": "101941660.0",
                "prevDayPx": "1.9573",
                "dayNtlVlm": "76578819.1965000033",
                "premium": "-0.0002047817",
                "oraclePx": "1.9533",
                "markPx": "1.9524",
                "midPx": "1.95285",
                "impactPxs": [
                    "1.952793",
                    "1.9529"
                ],
                "dayBaseVlm": "38616614.0"
            },
            {
                "funding": "-0.0000494224",
                "openInterest": "15281.294",
                "prevDayPx": "589.05",
                "dayNtlVlm": "3256727.6845900002",
                "premium": "-0.0008401622",
                "oraclePx": "579.65",
                "markPx": "579.15",
                "midPx": "579.155",
                "impactPxs": [
                    "579.035",
                    "579.163"
                ],
                "dayBaseVlm": "5559.223"
            },
            {
                "funding": "-0.0000262188",
                "openInterest": "4034096.8800000004",
                "prevDayPx": "1.6193",
                "dayNtlVlm": "1646722.6606069996",
                "premium": "-0.0006280224",
                "oraclePx": "1.5923",
                "markPx": "1.5907",
                "midPx": "1.5909",
                "impactPxs": [
                    "1.5904",
                    "1.5913"
                ],
                "dayBaseVlm": "1027441.8399999997"
            },
            {
                "funding": "-0.0000066623",
                "openInterest": "163982.0",
                "prevDayPx": "162.09",
                "dayNtlVlm": "4350337.5868999986",
                "premium": "-0.0004128497",
                "oraclePx": "161.56",
                "markPx": "161.47",
                "midPx": "161.485",
                "impactPxs": [
                    "161.4369",
                    "161.4933"
                ],
                "dayBaseVlm": "26629.57"
            },
            {
                "funding": "0.0000125",
                "openInterest": "32660.72",
                "prevDayPx": "25.043",
                "dayNtlVlm": "62667.55226",
                "premium": "0.0",
                "oraclePx": "25.0",
                "markPx": "24.977",
                "midPx": "24.986",
                "impactPxs": [
                    "24.9555",
                    "25.0162"
                ],
                "dayBaseVlm": "2477.32"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "1843.8",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "1828.8",
                "markPx": "1831.5",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "12872259.1999999993",
                "prevDayPx": "0.49211",
                "dayNtlVlm": "1501935.5638340004",
                "premium": "0.0",
                "oraclePx": "0.49405",
                "markPx": "0.49383",
                "midPx": "0.49403",
                "impactPxs": [
                    "0.49388",
                    "0.49419"
                ],
                "dayBaseVlm": "3010484.4999999995"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.65598",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.649",
                "markPx": "0.65538",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.041845",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.042203",
                "markPx": "0.041873",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.071",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.071068",
                "markPx": "0.071",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "7.636",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "6.053",
                "markPx": "7.636",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1656338.0",
                "prevDayPx": "0.064127",
                "dayNtlVlm": "169371.222763",
                "premium": "0.0",
                "oraclePx": "0.0647",
                "markPx": "0.064625",
                "midPx": "0.064638",
                "impactPxs": [
                    "0.064579",
                    "0.064718"
                ],
                "dayBaseVlm": "2619776.0"
            },
            {
                "funding": "-0.0000014673",
                "openInterest": "39648736.0",
                "prevDayPx": "0.31676",
                "dayNtlVlm": "1710173.5224400004",
                "premium": "-0.0003152934",
                "oraclePx": "0.30765",
                "markPx": "0.30741",
                "midPx": "0.307445",
                "impactPxs": [
                    "0.307344",
                    "0.307553"
                ],
                "dayBaseVlm": "5486224.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "422359174.0",
                "prevDayPx": "0.007859",
                "dayNtlVlm": "735342.4400909998",
                "premium": "0.0",
                "oraclePx": "0.007974",
                "markPx": "0.00797",
                "midPx": "0.007973",
                "impactPxs": [
                    "0.00797",
                    "0.007975"
                ],
                "dayBaseVlm": "92057335.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "4952759.0",
                "prevDayPx": "4.9699",
                "dayNtlVlm": "2555919.1749800001",
                "premium": "-0.0002251483",
                "oraclePx": "4.9745",
                "markPx": "4.9719",
                "midPx": "4.97235",
                "impactPxs": [
                    "4.9709",
                    "4.97338"
                ],
                "dayBaseVlm": "512611.8000000002"
            },
            {
                "funding": "-0.0000051102",
                "openInterest": "33895546.0",
                "prevDayPx": "0.10986",
                "dayNtlVlm": "1677718.3799300003",
                "premium": "-0.0004329802",
                "oraclePx": "0.10855",
                "markPx": "0.10847",
                "midPx": "0.108485",
                "impactPxs": [
                    "0.108437",
                    "0.108503"
                ],
                "dayBaseVlm": "15297084.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "2612724.3999999999",
                "prevDayPx": "0.62978",
                "dayNtlVlm": "157897.3698649999",
                "premium": "0.0",
                "oraclePx": "0.6225",
                "markPx": "0.62215",
                "midPx": "0.62242",
                "impactPxs": [
                    "0.62218",
                    "0.62267"
                ],
                "dayBaseVlm": "249975.2999999999"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.007459",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.004982",
                "markPx": "0.007459",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "4.72",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.47734",
                "markPx": "4.72",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.000604",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.000209",
                "markPx": "0.000621",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "1.0454",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "1.043",
                "markPx": "1.0415",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "5951346.3999999994",
                "prevDayPx": "1.6959",
                "dayNtlVlm": "3103212.2869600006",
                "premium": "0.0004780171",
                "oraclePx": "1.6945",
                "markPx": "1.6942",
                "midPx": "1.69575",
                "impactPxs": [
                    "1.69531",
                    "1.69604"
                ],
                "dayBaseVlm": "1778554.3"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.070239",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.070028",
                "markPx": "0.070043",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000005521",
                "openInterest": "3349323.2000000002",
                "prevDayPx": "1.9671",
                "dayNtlVlm": "990224.1594400003",
                "premium": "-0.000015015",
                "oraclePx": "1.998",
                "markPx": "1.9962",
                "midPx": "1.99705",
                "impactPxs": [
                    "1.99631",
                    "1.99797"
                ],
                "dayBaseVlm": "493003.7"
            },
            {
                "funding": "0.0000125",
                "openInterest": "36690.6",
                "prevDayPx": "6.4109",
                "dayNtlVlm": "204994.4087800001",
                "premium": "0.00019279",
                "oraclePx": "6.38",
                "markPx": "6.382",
                "midPx": "6.38415",
                "impactPxs": [
                    "6.38123",
                    "6.38808"
                ],
                "dayBaseVlm": "31679.3"
            },
            {
                "funding": "0.0000066973",
                "openInterest": "19186.3",
                "prevDayPx": "20.474",
                "dayNtlVlm": "187335.68577",
                "premium": "-0.0006812185",
                "oraclePx": "20.845",
                "markPx": "20.824",
                "midPx": "20.8235",
                "impactPxs": [
                    "20.8178",
                    "20.8308"
                ],
                "dayBaseVlm": "8963.82"
            },
            {
                "funding": "0.0000125",
                "openInterest": "144247.2",
                "prevDayPx": "0.49499",
                "dayNtlVlm": "32584.449088",
                "premium": "0.0",
                "oraclePx": "0.49123",
                "markPx": "0.49084",
                "midPx": "0.49086",
                "impactPxs": [
                    "0.48951",
                    "0.49219"
                ],
                "dayBaseVlm": "65884.4"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.07116",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.071385",
                "markPx": "0.071161",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.0558",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.0558",
                "markPx": "0.055683",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.018338",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.018338",
                "markPx": "0.018328",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0000003981",
                "openInterest": "1751896.0",
                "prevDayPx": "0.26127",
                "dayNtlVlm": "61648.47355",
                "premium": "-0.0014975105",
                "oraclePx": "0.2611",
                "markPx": "0.26042",
                "midPx": "0.260405",
                "impactPxs": [
                    "0.260138",
                    "0.260709"
                ],
                "dayBaseVlm": "234813.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.38457",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.38349",
                "markPx": "0.38287",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.066",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.06598",
                "markPx": "0.066",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.15473",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.15423",
                "markPx": "0.15473",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "12503540.0",
                "prevDayPx": "0.022559",
                "dayNtlVlm": "147978.8873799999",
                "premium": "-0.000179051",
                "oraclePx": "0.02234",
                "markPx": "0.022318",
                "midPx": "0.022311",
                "impactPxs": [
                    "0.022289",
                    "0.022336"
                ],
                "dayBaseVlm": "6517028.0"
            },
            {
                "funding": "-0.0000124352",
                "openInterest": "24854114.0",
                "prevDayPx": "0.042532",
                "dayNtlVlm": "126382.850143",
                "premium": "-0.0005022722",
                "oraclePx": "0.04181",
                "markPx": "0.041765",
                "midPx": "0.041771",
                "impactPxs": [
                    "0.041749",
                    "0.041789"
                ],
                "dayBaseVlm": "2968226.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.01939",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.019393",
                "markPx": "0.01939",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0000030813",
                "openInterest": "25898856.0",
                "prevDayPx": "0.033812",
                "dayNtlVlm": "405730.680706",
                "premium": "0.0",
                "oraclePx": "0.03359",
                "markPx": "0.033559",
                "midPx": "0.033582",
                "impactPxs": [
                    "0.033567",
                    "0.033598"
                ],
                "dayBaseVlm": "11995232.0"
            },
            {
                "funding": "0.0000008112",
                "openInterest": "5734641.4000000004",
                "prevDayPx": "0.49624",
                "dayNtlVlm": "1244672.8118550007",
                "premium": "-0.0002046664",
                "oraclePx": "0.4886",
                "markPx": "0.48823",
                "midPx": "0.48828",
                "impactPxs": [
                    "0.48817",
                    "0.4885"
                ],
                "dayBaseVlm": "2515211.5000000005"
            },
            {
                "funding": "0.0000125",
                "openInterest": "37313.32",
                "prevDayPx": "18.892",
                "dayNtlVlm": "61481.04893",
                "premium": "0.0",
                "oraclePx": "18.565",
                "markPx": "18.55",
                "midPx": "18.553",
                "impactPxs": [
                    "18.5352",
                    "18.5661"
                ],
                "dayBaseVlm": "3239.94"
            },
            {
                "funding": "-0.000036557",
                "openInterest": "57605194.0",
                "prevDayPx": "0.36408",
                "dayNtlVlm": "3577236.763530001",
                "premium": "-0.0004876454",
                "oraclePx": "0.36707",
                "markPx": "0.36676",
                "midPx": "0.3668",
                "impactPxs": [
                    "0.366727",
                    "0.366891"
                ],
                "dayBaseVlm": "9735888.0"
            },
            {
                "funding": "0.0000041916",
                "openInterest": "6402046.3999999994",
                "prevDayPx": "1.6074",
                "dayNtlVlm": "1918180.1820899996",
                "premium": "-0.0004781639",
                "oraclePx": "1.5685",
                "markPx": "1.5671",
                "midPx": "1.5676",
                "impactPxs": [
                    "1.56712",
                    "1.56775"
                ],
                "dayBaseVlm": "1209971.8999999994"
            },
            {
                "funding": "-0.0000216838",
                "openInterest": "3736234.0",
                "prevDayPx": "0.083153",
                "dayNtlVlm": "169922.217579",
                "premium": "-0.0004191617",
                "oraclePx": "0.0835",
                "markPx": "0.08335",
                "midPx": "0.08332",
                "impactPxs": [
                    "0.083148",
                    "0.083465"
                ],
                "dayBaseVlm": "1989144.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "6266640.0",
                "prevDayPx": "0.060209",
                "dayNtlVlm": "107297.522058",
                "premium": "0.0",
                "oraclePx": "0.058008",
                "markPx": "0.05789",
                "midPx": "0.057942",
                "impactPxs": [
                    "0.057773",
                    "0.05803"
                ],
                "dayBaseVlm": "1797210.0"
            },
            {
                "funding": "0.0000067375",
                "openInterest": "266890.6",
                "prevDayPx": "1.9322",
                "dayNtlVlm": "79407.2858900001",
                "premium": "-0.000082453",
                "oraclePx": "1.9405",
                "markPx": "1.9376",
                "midPx": "1.93765",
                "impactPxs": [
                    "1.93527",
                    "1.94034"
                ],
                "dayBaseVlm": "40841.0"
            },
            {
                "funding": "-0.000028102",
                "openInterest": "2852476.0",
                "prevDayPx": "1.8796",
                "dayNtlVlm": "2669813.6469999999",
                "premium": "0.0",
                "oraclePx": "1.987",
                "markPx": "1.9849",
                "midPx": "1.9857",
                "impactPxs": [
                    "1.984727",
                    "1.987898"
                ],
                "dayBaseVlm": "1357973.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.1537",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.15385",
                "markPx": "0.15387",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0002401199",
                "openInterest": "17626934.0",
                "prevDayPx": "0.24324",
                "dayNtlVlm": "4737874.3634299999",
                "premium": "-0.001546073",
                "oraclePx": "0.24255",
                "markPx": "0.24195",
                "midPx": "0.24205",
                "impactPxs": [
                    "0.241868",
                    "0.242175"
                ],
                "dayBaseVlm": "19330470.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "1.5848",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.0416",
                "markPx": "1.5829",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "6516895.7999999998",
                "prevDayPx": "1.5593",
                "dayNtlVlm": "2059015.2737999994",
                "premium": "-0.0002564925",
                "oraclePx": "1.5595",
                "markPx": "1.5585",
                "midPx": "1.5587",
                "impactPxs": [
                    "1.55819",
                    "1.5591"
                ],
                "dayBaseVlm": "1304651.6999999995"
            },
            {
                "funding": "-0.0011457372",
                "openInterest": "1103520828.0",
                "prevDayPx": "0.001005",
                "dayNtlVlm": "2050752.2700719999",
                "premium": "-0.0031620553",
                "oraclePx": "0.001265",
                "markPx": "0.001258",
                "midPx": "0.001259",
                "impactPxs": [
                    "0.001258",
                    "0.001261"
                ],
                "dayBaseVlm": "1647244203.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "82453.84",
                "prevDayPx": "4.602",
                "dayNtlVlm": "343532.533694",
                "premium": "0.0",
                "oraclePx": "4.377",
                "markPx": "4.3742",
                "midPx": "4.3743",
                "impactPxs": [
                    "4.3714",
                    "4.377"
                ],
                "dayBaseVlm": "77292.7"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.89235",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.88959",
                "markPx": "0.88823",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000078315",
                "openInterest": "143275.48",
                "prevDayPx": "3.6936",
                "dayNtlVlm": "100642.289164",
                "premium": "0.0",
                "oraclePx": "3.719",
                "markPx": "3.716",
                "midPx": "3.7169",
                "impactPxs": [
                    "3.7122",
                    "3.7199"
                ],
                "dayBaseVlm": "26882.2"
            },
            {
                "funding": "0.0000063479",
                "openInterest": "99836.14",
                "prevDayPx": "11.985",
                "dayNtlVlm": "1311024.4899600002",
                "premium": "-0.0001360809",
                "oraclePx": "10.288",
                "markPx": "10.277",
                "midPx": "10.28",
                "impactPxs": [
                    "10.269",
                    "10.2866"
                ],
                "dayBaseVlm": "117258.44"
            },
            {
                "funding": "-0.0000278141",
                "openInterest": "2302973.8000000003",
                "prevDayPx": "1.3922",
                "dayNtlVlm": "522956.04525",
                "premium": "-0.0001399632",
                "oraclePx": "1.3575",
                "markPx": "1.3558",
                "midPx": "1.3566",
                "impactPxs": [
                    "1.35589",
                    "1.35731"
                ],
                "dayBaseVlm": "379626.9999999998"
            },
            {
                "funding": "0.0000023968",
                "openInterest": "16173076.0",
                "prevDayPx": "0.059261",
                "dayNtlVlm": "391263.826646",
                "premium": "-0.0004771236",
                "oraclePx": "0.058685",
                "markPx": "0.058625",
                "midPx": "0.058627",
                "impactPxs": [
                    "0.058603",
                    "0.058657"
                ],
                "dayBaseVlm": "6571743.0"
            },
            {
                "funding": "-0.0000082797",
                "openInterest": "1131202.0",
                "prevDayPx": "0.30581",
                "dayNtlVlm": "180866.1744749999",
                "premium": "-0.0005774969",
                "oraclePx": "0.31169",
                "markPx": "0.3113",
                "midPx": "0.31133",
                "impactPxs": [
                    "0.31117",
                    "0.31151"
                ],
                "dayBaseVlm": "582066.4999999999"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "12.128",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "12.14",
                "markPx": "12.118",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "4560283.7999999998",
                "prevDayPx": "0.25721",
                "dayNtlVlm": "367427.4590419999",
                "premium": "0.0",
                "oraclePx": "0.2594",
                "markPx": "0.25925",
                "midPx": "0.25928",
                "impactPxs": [
                    "0.25902",
                    "0.25976"
                ],
                "dayBaseVlm": "1411113.9000000004"
            },
            {
                "funding": "0.0000055952",
                "openInterest": "395098196.0",
                "prevDayPx": "0.009209",
                "dayNtlVlm": "1998256.7722440006",
                "premium": "-0.0003259452",
                "oraclePx": "0.009204",
                "markPx": "0.009196",
                "midPx": "0.009198",
                "impactPxs": [
                    "0.009196",
                    "0.009201"
                ],
                "dayBaseVlm": "216307233.0"
            },
            {
                "funding": "-0.0000171384",
                "openInterest": "21215446.0",
                "prevDayPx": "0.017504",
                "dayNtlVlm": "130843.666127",
                "premium": "0.0",
                "oraclePx": "0.017045",
                "markPx": "0.017015",
                "midPx": "0.017021",
                "impactPxs": [
                    "0.017002",
                    "0.01705"
                ],
                "dayBaseVlm": "7502055.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "2709240.0",
                "prevDayPx": "0.19907",
                "dayNtlVlm": "108749.41088",
                "premium": "-0.0004683479",
                "oraclePx": "0.1943",
                "markPx": "0.19398",
                "midPx": "0.194035",
                "impactPxs": [
                    "0.193883",
                    "0.194209"
                ],
                "dayBaseVlm": "547706.0"
            },
            {
                "funding": "-0.0000339693",
                "openInterest": "29146182.0",
                "prevDayPx": "0.006574",
                "dayNtlVlm": "81088.131523",
                "premium": "0.0",
                "oraclePx": "0.006634",
                "markPx": "0.006631",
                "midPx": "0.006632",
                "impactPxs": [
                    "0.006628",
                    "0.006636"
                ],
                "dayBaseVlm": "12132014.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "6.6683",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "2518.4",
                "markPx": "6.5609",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000056945",
                "openInterest": "25112378.0",
                "prevDayPx": "0.19992",
                "dayNtlVlm": "758649.5951500001",
                "premium": "-0.000241631",
                "oraclePx": "0.19865",
                "markPx": "0.19845",
                "midPx": "0.19854",
                "impactPxs": [
                    "0.198461",
                    "0.198602"
                ],
                "dayBaseVlm": "3772025.0"
            },
            {
                "funding": "-0.0000693581",
                "openInterest": "24875082.0",
                "prevDayPx": "0.039064",
                "dayNtlVlm": "138275.065414",
                "premium": "-0.000233342",
                "oraclePx": "0.03857",
                "markPx": "0.038543",
                "midPx": "0.038533",
                "impactPxs": [
                    "0.038492",
                    "0.038561"
                ],
                "dayBaseVlm": "3517528.0"
            },
            {
                "funding": "-0.0000128611",
                "openInterest": "88102672.0",
                "prevDayPx": "0.00239",
                "dayNtlVlm": "36107.632283",
                "premium": "-0.0004135649",
                "oraclePx": "0.002418",
                "markPx": "0.002414",
                "midPx": "0.002415",
                "impactPxs": [
                    "0.002413",
                    "0.002417"
                ],
                "dayBaseVlm": "14922994.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "99642642.0",
                "prevDayPx": "0.006668",
                "dayNtlVlm": "190392.073666",
                "premium": "0.0",
                "oraclePx": "0.006626",
                "markPx": "0.006616",
                "midPx": "0.006618",
                "impactPxs": [
                    "0.006611",
                    "0.006631"
                ],
                "dayBaseVlm": "28564648.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "2136088.0",
                "prevDayPx": "0.35864",
                "dayNtlVlm": "159954.6614299999",
                "premium": "0.0",
                "oraclePx": "0.35155",
                "markPx": "0.35135",
                "midPx": "0.351405",
                "impactPxs": [
                    "0.351082",
                    "0.351649"
                ],
                "dayBaseVlm": "446928.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.12371",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.12385",
                "markPx": "0.1238",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.000184148",
                "openInterest": "1487171.7",
                "prevDayPx": "0.2503",
                "dayNtlVlm": "266236.888701",
                "premium": "-0.0027580772",
                "oraclePx": "0.2538",
                "markPx": "0.253",
                "midPx": "0.253",
                "impactPxs": [
                    "0.2529",
                    "0.2531"
                ],
                "dayBaseVlm": "1051174.7499999998"
            },
            {
                "funding": "0.0000125",
                "openInterest": "8224368.0",
                "prevDayPx": "0.024311",
                "dayNtlVlm": "205314.432572",
                "premium": "0.0",
                "oraclePx": "0.025715",
                "markPx": "0.025708",
                "midPx": "0.025718",
                "impactPxs": [
                    "0.025683",
                    "0.02576"
                ],
                "dayBaseVlm": "7872741.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "20277628.0",
                "prevDayPx": "0.34186",
                "dayNtlVlm": "1257731.3060400004",
                "premium": "0.0002028104",
                "oraclePx": "0.34515",
                "markPx": "0.34495",
                "midPx": "0.345315",
                "impactPxs": [
                    "0.34522",
                    "0.345416"
                ],
                "dayBaseVlm": "3626200.0"
            },
            {
                "funding": "-0.000022211",
                "openInterest": "614194.4000000001",
                "prevDayPx": "2.0157",
                "dayNtlVlm": "311322.4283899999",
                "premium": "-0.0012734686",
                "oraclePx": "1.9867",
                "markPx": "1.9835",
                "midPx": "1.98355",
                "impactPxs": [
                    "1.98274",
                    "1.98417"
                ],
                "dayBaseVlm": "154611.7"
            },
            {
                "funding": "0.0000125",
                "openInterest": "17339832.0",
                "prevDayPx": "0.009526",
                "dayNtlVlm": "169431.0265450001",
                "premium": "-0.0001026694",
                "oraclePx": "0.00974",
                "markPx": "0.009727",
                "midPx": "0.009728",
                "impactPxs": [
                    "0.009711",
                    "0.009739"
                ],
                "dayBaseVlm": "17609205.0"
            },
            {
                "funding": "-0.0000072674",
                "openInterest": "31845.36",
                "prevDayPx": "9.5016",
                "dayNtlVlm": "124613.999651",
                "premium": "-0.0006153193",
                "oraclePx": "9.426",
                "markPx": "9.4188",
                "midPx": "9.4176",
                "impactPxs": [
                    "9.4144",
                    "9.4202"
                ],
                "dayBaseVlm": "13071.79"
            },
            {
                "funding": "-0.0000150968",
                "openInterest": "150477.06",
                "prevDayPx": "11.857",
                "dayNtlVlm": "344226.8862099999",
                "premium": "-0.0002696781",
                "oraclePx": "11.866",
                "markPx": "11.856",
                "midPx": "11.8565",
                "impactPxs": [
                    "11.8504",
                    "11.8628"
                ],
                "dayBaseVlm": "28869.08"
            },
            {
                "funding": "-0.0000376908",
                "openInterest": "53374541.200000003",
                "prevDayPx": "0.01638",
                "dayNtlVlm": "145611.923088",
                "premium": "-0.000618047",
                "oraclePx": "0.01618",
                "markPx": "0.01615",
                "midPx": "0.01615",
                "impactPxs": [
                    "0.01613",
                    "0.01617"
                ],
                "dayBaseVlm": "8923974.2999999989"
            },
            {
                "funding": "0.0000125",
                "openInterest": "2060936.2",
                "prevDayPx": "0.0784",
                "dayNtlVlm": "84968.571159",
                "premium": "0.0",
                "oraclePx": "0.0778",
                "markPx": "0.07771",
                "midPx": "0.07773",
                "impactPxs": [
                    "0.07761",
                    "0.07798"
                ],
                "dayBaseVlm": "1063442.8000000003"
            },
            {
                "funding": "0.0000125",
                "openInterest": "681358.4",
                "prevDayPx": "0.72433",
                "dayNtlVlm": "49613.568902",
                "premium": "-0.0001790634",
                "oraclePx": "0.726",
                "markPx": "0.72532",
                "midPx": "0.72536",
                "impactPxs": [
                    "0.72484",
                    "0.72587"
                ],
                "dayBaseVlm": "67544.8"
            },
            {
                "funding": "-0.0000018433",
                "openInterest": "11714268.0",
                "prevDayPx": "0.33616",
                "dayNtlVlm": "963592.9611799996",
                "premium": "-0.0002159323",
                "oraclePx": "0.3427",
                "markPx": "0.34239",
                "midPx": "0.342455",
                "impactPxs": [
                    "0.34229",
                    "0.342626"
                ],
                "dayBaseVlm": "2813205.0"
            },
            {
                "funding": "0.0000063626",
                "openInterest": "47519942.0",
                "prevDayPx": "0.011943",
                "dayNtlVlm": "183624.57029",
                "premium": "0.0",
                "oraclePx": "0.012005",
                "markPx": "0.01199",
                "midPx": "0.011996",
                "impactPxs": [
                    "0.011985",
                    "0.012007"
                ],
                "dayBaseVlm": "15173507.0"
            },
            {
                "funding": "-0.0000181731",
                "openInterest": "11417434.4000000004",
                "prevDayPx": "0.0728",
                "dayNtlVlm": "99151.411164",
                "premium": "-0.0001398601",
                "oraclePx": "0.0715",
                "markPx": "0.0714",
                "midPx": "0.07144",
                "impactPxs": [
                    "0.07139",
                    "0.07149"
                ],
                "dayBaseVlm": "1352351.0999999999"
            },
            {
                "funding": "0.0000008714",
                "openInterest": "7833353.0",
                "prevDayPx": "0.0622",
                "dayNtlVlm": "124942.67681",
                "premium": "-0.0009590793",
                "oraclePx": "0.06256",
                "markPx": "0.06245",
                "midPx": "0.06242",
                "impactPxs": [
                    "0.06236",
                    "0.0625"
                ],
                "dayBaseVlm": "1976039.8999999994"
            },
            {
                "funding": "0.0000125",
                "openInterest": "28303264.8000000007",
                "prevDayPx": "0.05568",
                "dayNtlVlm": "119243.399725",
                "premium": "0.0007335412",
                "oraclePx": "0.05453",
                "markPx": "0.05463",
                "midPx": "0.05465",
                "impactPxs": [
                    "0.05457",
                    "0.05473"
                ],
                "dayBaseVlm": "2111512.5999999996"
            },
            {
                "funding": "0.0000125",
                "openInterest": "42023628.3999999985",
                "prevDayPx": "0.03319",
                "dayNtlVlm": "182011.723613",
                "premium": "0.0",
                "oraclePx": "0.03284",
                "markPx": "0.03282",
                "midPx": "0.03283",
                "impactPxs": [
                    "0.03281",
                    "0.03284"
                ],
                "dayBaseVlm": "5446399.2000000002"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "3012.0",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "1706.6",
                "markPx": "3026.3",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0000643203",
                "openInterest": "60501205.799999997",
                "prevDayPx": "0.08274",
                "dayNtlVlm": "602349.5432650003",
                "premium": "-0.0004991265",
                "oraclePx": "0.08014",
                "markPx": "0.08003",
                "midPx": "0.08006",
                "impactPxs": [
                    "0.08003",
                    "0.0801"
                ],
                "dayBaseVlm": "7380644.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.024008",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.024025",
                "markPx": "0.024027",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.12565",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.1257",
                "markPx": "0.12555",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "57306.916",
                "prevDayPx": "247.72",
                "dayNtlVlm": "3828396.9056399995",
                "premium": "0.0",
                "oraclePx": "244.55",
                "markPx": "244.45",
                "midPx": "244.53",
                "impactPxs": [
                    "244.417",
                    "244.581"
                ],
                "dayBaseVlm": "15265.204"
            },
            {
                "funding": "0.0000113258",
                "openInterest": "227216.72",
                "prevDayPx": "3.7675",
                "dayNtlVlm": "473697.8040639999",
                "premium": "0.0",
                "oraclePx": "3.76",
                "markPx": "3.7558",
                "midPx": "3.7583",
                "impactPxs": [
                    "3.7549",
                    "3.7605"
                ],
                "dayBaseVlm": "124500.05"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.015813",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.015815",
                "markPx": "0.015813",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000025501",
                "openInterest": "13550440.0",
                "prevDayPx": "0.044087",
                "dayNtlVlm": "270161.963765",
                "premium": "-0.0001342883",
                "oraclePx": "0.04468",
                "markPx": "0.044638",
                "midPx": "0.044649",
                "impactPxs": [
                    "0.04463",
                    "0.044674"
                ],
                "dayBaseVlm": "6037249.0"
            },
            {
                "funding": "0.0000095651",
                "openInterest": "261769064.0",
                "prevDayPx": "0.000594",
                "dayNtlVlm": "34112.809892",
                "premium": "0.0",
                "oraclePx": "0.000618",
                "markPx": "0.000617",
                "midPx": "0.000618",
                "impactPxs": [
                    "0.000616",
                    "0.000619"
                ],
                "dayBaseVlm": "56109487.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "5419964.7999999998",
                "prevDayPx": "0.65196",
                "dayNtlVlm": "430707.1727329998",
                "premium": "-0.0003162805",
                "oraclePx": "0.63235",
                "markPx": "0.6318",
                "midPx": "0.63194",
                "impactPxs": [
                    "0.63169",
                    "0.63215"
                ],
                "dayBaseVlm": "668358.5"
            },
            {
                "funding": "-0.0000023544",
                "openInterest": "143931490.0",
                "prevDayPx": "0.19461",
                "dayNtlVlm": "4819009.9141199989",
                "premium": "0.0",
                "oraclePx": "0.18985",
                "markPx": "0.18975",
                "midPx": "0.18982",
                "impactPxs": [
                    "0.189771",
                    "0.189855"
                ],
                "dayBaseVlm": "25028822.0"
            },
            {
                "funding": "-0.000051111",
                "openInterest": "3743125.6000000001",
                "prevDayPx": "0.92123",
                "dayNtlVlm": "584557.4671039999",
                "premium": "-0.0007582093",
                "oraclePx": "0.89685",
                "markPx": "0.89595",
                "midPx": "0.89592",
                "impactPxs": [
                    "0.89558",
                    "0.89617"
                ],
                "dayBaseVlm": "640519.5"
            },
            {
                "funding": "0.0000125",
                "openInterest": "3652458.0",
                "prevDayPx": "0.0643",
                "dayNtlVlm": "116494.331937",
                "premium": "-0.0004711795",
                "oraclePx": "0.06367",
                "markPx": "0.06358",
                "midPx": "0.06358",
                "impactPxs": [
                    "0.06353",
                    "0.06364"
                ],
                "dayBaseVlm": "1767740.0999999999"
            },
            {
                "funding": "0.0000125",
                "openInterest": "13017731.4000000004",
                "prevDayPx": "0.05914",
                "dayNtlVlm": "200513.045008",
                "premium": "0.0",
                "oraclePx": "0.0585",
                "markPx": "0.05847",
                "midPx": "0.0585",
                "impactPxs": [
                    "0.05848",
                    "0.05852"
                ],
                "dayBaseVlm": "3363727.6000000006"
            },
            {
                "funding": "0.0000125",
                "openInterest": "3668314.0",
                "prevDayPx": "0.21056",
                "dayNtlVlm": "1173899.7494499995",
                "premium": "0.0",
                "oraclePx": "0.172",
                "markPx": "0.17185",
                "midPx": "0.171865",
                "impactPxs": [
                    "0.171707",
                    "0.172028"
                ],
                "dayBaseVlm": "6492483.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "24257116.0",
                "prevDayPx": "0.10913",
                "dayNtlVlm": "642751.4042100001",
                "premium": "0.0",
                "oraclePx": "0.10878",
                "markPx": "0.10873",
                "midPx": "0.108745",
                "impactPxs": [
                    "0.10869",
                    "0.108799"
                ],
                "dayBaseVlm": "5831969.0"
            },
            {
                "funding": "-0.0000078777",
                "openInterest": "14112436.0",
                "prevDayPx": "0.080621",
                "dayNtlVlm": "640577.566908",
                "premium": "0.0",
                "oraclePx": "0.080165",
                "markPx": "0.080088",
                "midPx": "0.080135",
                "impactPxs": [
                    "0.080079",
                    "0.080183"
                ],
                "dayBaseVlm": "7875373.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "4.1915",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "4.1948",
                "markPx": "4.1915",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0000270333",
                "openInterest": "8794784.2400000002",
                "prevDayPx": "0.3439",
                "dayNtlVlm": "936785.917547",
                "premium": "-0.0002957705",
                "oraclePx": "0.3381",
                "markPx": "0.3379",
                "midPx": "0.3378",
                "impactPxs": [
                    "0.3377",
                    "0.338"
                ],
                "dayBaseVlm": "2738894.7199999988"
            },
            {
                "funding": "-0.0000438669",
                "openInterest": "111173690.0",
                "prevDayPx": "0.004712",
                "dayNtlVlm": "74788.825937",
                "premium": "-0.0004335573",
                "oraclePx": "0.004613",
                "markPx": "0.004606",
                "midPx": "0.004607",
                "impactPxs": [
                    "0.004605",
                    "0.004611"
                ],
                "dayBaseVlm": "16016104.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "304507772.0",
                "prevDayPx": "0.000541",
                "dayNtlVlm": "28438.341937",
                "premium": "0.0",
                "oraclePx": "0.000549",
                "markPx": "0.000548",
                "midPx": "0.000548",
                "impactPxs": [
                    "0.000547",
                    "0.000549"
                ],
                "dayBaseVlm": "52159737.0"
            },
            {
                "funding": "-0.000064385",
                "openInterest": "300906010.0",
                "prevDayPx": "0.001587",
                "dayNtlVlm": "197608.8411930001",
                "premium": "-0.0006293266",
                "oraclePx": "0.001589",
                "markPx": "0.001586",
                "midPx": "0.001587",
                "impactPxs": [
                    "0.001585",
                    "0.001588"
                ],
                "dayBaseVlm": "123081438.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "61531172.0",
                "prevDayPx": "0.014757",
                "dayNtlVlm": "161812.194915",
                "premium": "-0.0002076125",
                "oraclePx": "0.01445",
                "markPx": "0.014438",
                "midPx": "0.01444",
                "impactPxs": [
                    "0.014416",
                    "0.014447"
                ],
                "dayBaseVlm": "10932058.0"
            },
            {
                "funding": "-0.000047418",
                "openInterest": "2259776.2000000002",
                "prevDayPx": "0.1501",
                "dayNtlVlm": "322285.2752029999",
                "premium": "0.0",
                "oraclePx": "0.15048",
                "markPx": "0.15025",
                "midPx": "0.15027",
                "impactPxs": [
                    "0.15006",
                    "0.15048"
                ],
                "dayBaseVlm": "2101114.0000000005"
            },
            {
                "funding": "-0.0000048429",
                "openInterest": "27060306.0",
                "prevDayPx": "0.032602",
                "dayNtlVlm": "273993.5154229999",
                "premium": "0.0",
                "oraclePx": "0.03067",
                "markPx": "0.03064",
                "midPx": "0.030665",
                "impactPxs": [
                    "0.030637",
                    "0.030685"
                ],
                "dayBaseVlm": "8573942.0"
            },
            {
                "funding": "-0.0000630505",
                "openInterest": "1365732718.0",
                "prevDayPx": "0.000786",
                "dayNtlVlm": "108508.104699",
                "premium": "0.0",
                "oraclePx": "0.000745",
                "markPx": "0.000743",
                "midPx": "0.000743",
                "impactPxs": [
                    "0.000742",
                    "0.000745"
                ],
                "dayBaseVlm": "139161090.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.12493",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.1247",
                "markPx": "0.12466",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000111844",
                "openInterest": "230076068.0",
                "prevDayPx": "0.000954",
                "dayNtlVlm": "108180.618878",
                "premium": "0.0",
                "oraclePx": "0.000927",
                "markPx": "0.000926",
                "midPx": "0.000925",
                "impactPxs": [
                    "0.000924",
                    "0.000927"
                ],
                "dayBaseVlm": "113236974.0"
            },
            {
                "funding": "0.0000110253",
                "openInterest": "786638.4",
                "prevDayPx": "2.0438",
                "dayNtlVlm": "510213.2565799998",
                "premium": "-0.0000545229",
                "oraclePx": "2.0175",
                "markPx": "2.0155",
                "midPx": "2.0156",
                "impactPxs": [
                    "2.0129",
                    "2.01739"
                ],
                "dayBaseVlm": "248060.9"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.1367",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.13675",
                "markPx": "0.13659",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0000103838",
                "openInterest": "22928672.0",
                "prevDayPx": "0.13662",
                "dayNtlVlm": "1610332.4159699995",
                "premium": "-0.0007102804",
                "oraclePx": "0.13375",
                "markPx": "0.13364",
                "midPx": "0.133595",
                "impactPxs": [
                    "0.133544",
                    "0.133655"
                ],
                "dayBaseVlm": "11880107.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.083021",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.08275",
                "markPx": "0.082821",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "5513984.0",
                "prevDayPx": "0.12292",
                "dayNtlVlm": "313789.12914",
                "premium": "-0.0009912536",
                "oraclePx": "0.12005",
                "markPx": "0.11989",
                "midPx": "0.119875",
                "impactPxs": [
                    "0.119843",
                    "0.119931"
                ],
                "dayBaseVlm": "2546445.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "8429545178.0",
                "prevDayPx": "0.000229",
                "dayNtlVlm": "51741.420705",
                "premium": "0.0",
                "oraclePx": "0.000232",
                "markPx": "0.000232",
                "midPx": "0.000233",
                "impactPxs": [
                    "0.000232",
                    "0.000234"
                ],
                "dayBaseVlm": "224093752.0"
            },
            {
                "funding": "0.0000090873",
                "openInterest": "2264228.3999999999",
                "prevDayPx": "0.07657",
                "dayNtlVlm": "343120.615325",
                "premium": "-0.0010201479",
                "oraclePx": "0.07842",
                "markPx": "0.07827",
                "midPx": "0.07824",
                "impactPxs": [
                    "0.07814",
                    "0.07834"
                ],
                "dayBaseVlm": "4349907.1000000006"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.018168",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.017705",
                "markPx": "0.01808",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0000220673",
                "openInterest": "3537606.0",
                "prevDayPx": "0.11699",
                "dayNtlVlm": "205747.118183",
                "premium": "0.0",
                "oraclePx": "0.11927",
                "markPx": "0.11907",
                "midPx": "0.1192",
                "impactPxs": [
                    "0.11908",
                    "0.11927"
                ],
                "dayBaseVlm": "1734979.8"
            },
            {
                "funding": "-0.0000769079",
                "openInterest": "22137146.0",
                "prevDayPx": "0.032918",
                "dayNtlVlm": "633826.628438",
                "premium": "-0.0004019168",
                "oraclePx": "0.032345",
                "markPx": "0.032272",
                "midPx": "0.032285",
                "impactPxs": [
                    "0.032208",
                    "0.032332"
                ],
                "dayBaseVlm": "19279950.0"
            },
            {
                "funding": "-0.0000423251",
                "openInterest": "23020212.0",
                "prevDayPx": "0.068592",
                "dayNtlVlm": "774465.9552859999",
                "premium": "-0.0007143586",
                "oraclePx": "0.068593",
                "markPx": "0.068485",
                "midPx": "0.068513",
                "impactPxs": [
                    "0.068497",
                    "0.068544"
                ],
                "dayBaseVlm": "11141772.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "2772484.0",
                "prevDayPx": "0.29474",
                "dayNtlVlm": "131378.096325",
                "premium": "0.0",
                "oraclePx": "0.28643",
                "markPx": "0.28635",
                "midPx": "0.28644",
                "impactPxs": [
                    "0.28619",
                    "0.28659"
                ],
                "dayBaseVlm": "450830.5000000002"
            },
            {
                "funding": "0.0000114984",
                "openInterest": "45935452.0",
                "prevDayPx": "0.059432",
                "dayNtlVlm": "555539.262948",
                "premium": "0.0",
                "oraclePx": "0.054664",
                "markPx": "0.054516",
                "midPx": "0.054584",
                "impactPxs": [
                    "0.054413",
                    "0.054732"
                ],
                "dayBaseVlm": "9421146.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "8689713.0",
                "prevDayPx": "0.07407",
                "dayNtlVlm": "295634.188017",
                "premium": "0.0",
                "oraclePx": "0.07413",
                "markPx": "0.07406",
                "midPx": "0.07409",
                "impactPxs": [
                    "0.07403",
                    "0.07414"
                ],
                "dayBaseVlm": "3953800.0"
            },
            {
                "funding": "-0.0000026704",
                "openInterest": "15228526.0",
                "prevDayPx": "0.21318",
                "dayNtlVlm": "414847.3991299999",
                "premium": "-0.0001168443",
                "oraclePx": "0.21396",
                "markPx": "0.21381",
                "midPx": "0.213865",
                "impactPxs": [
                    "0.213759",
                    "0.213935"
                ],
                "dayBaseVlm": "1923520.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "41314308.0",
                "prevDayPx": "0.018636",
                "dayNtlVlm": "384331.412713",
                "premium": "0.0",
                "oraclePx": "0.017895",
                "markPx": "0.017905",
                "midPx": "0.017908",
                "impactPxs": [
                    "0.01787",
                    "0.017935"
                ],
                "dayBaseVlm": "20774199.0"
            },
            {
                "funding": "-0.0000261523",
                "openInterest": "4934932.0",
                "prevDayPx": "0.13582",
                "dayNtlVlm": "743665.3024200001",
                "premium": "0.0",
                "oraclePx": "0.14035",
                "markPx": "0.14022",
                "midPx": "0.140335",
                "impactPxs": [
                    "0.140113",
                    "0.140473"
                ],
                "dayBaseVlm": "5423841.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "3545656.0",
                "prevDayPx": "0.085425",
                "dayNtlVlm": "32092.737627",
                "premium": "0.0",
                "oraclePx": "0.085",
                "markPx": "0.08485",
                "midPx": "0.084926",
                "impactPxs": [
                    "0.084785",
                    "0.085048"
                ],
                "dayBaseVlm": "376795.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "8707074.0",
                "prevDayPx": "0.11841",
                "dayNtlVlm": "227764.42938",
                "premium": "0.0",
                "oraclePx": "0.11865",
                "markPx": "0.11851",
                "midPx": "0.11855",
                "impactPxs": [
                    "0.118459",
                    "0.118667"
                ],
                "dayBaseVlm": "1896606.0"
            },
            {
                "funding": "0.0000259539",
                "openInterest": "27523052.7800000086",
                "prevDayPx": "23.79",
                "dayNtlVlm": "149418867.5994799733",
                "premium": "0.000798929",
                "oraclePx": "23.156",
                "markPx": "23.177",
                "midPx": "23.1775",
                "impactPxs": [
                    "23.1745",
                    "23.1839"
                ],
                "dayBaseVlm": "6296145.1700000027"
            },
            {
                "funding": "-0.0001075806",
                "openInterest": "2276212.6000000001",
                "prevDayPx": "0.25492",
                "dayNtlVlm": "899556.1644889999",
                "premium": "-0.000862423",
                "oraclePx": "0.2435",
                "markPx": "0.24299",
                "midPx": "0.24301",
                "impactPxs": [
                    "0.2429",
                    "0.24329"
                ],
                "dayBaseVlm": "3623790.1999999993"
            },
            {
                "funding": "-0.0000090717",
                "openInterest": "59777020.0",
                "prevDayPx": "0.036753",
                "dayNtlVlm": "434696.9531329998",
                "premium": "-0.0003165012",
                "oraclePx": "0.034755",
                "markPx": "0.034718",
                "midPx": "0.034729",
                "impactPxs": [
                    "0.034715",
                    "0.034744"
                ],
                "dayBaseVlm": "12017913.0"
            },
            {
                "funding": "0.0000041856",
                "openInterest": "10945977.5999999996",
                "prevDayPx": "0.8545",
                "dayNtlVlm": "3778031.1669709999",
                "premium": "-0.0003683029",
                "oraclePx": "0.86885",
                "markPx": "0.86825",
                "midPx": "0.86826",
                "impactPxs": [
                    "0.868",
                    "0.86853"
                ],
                "dayBaseVlm": "4334733.8000000007"
            },
            {
                "funding": "-0.0000305518",
                "openInterest": "344940336.0",
                "prevDayPx": "0.010178",
                "dayNtlVlm": "2143617.0373689993",
                "premium": "-0.000297118",
                "oraclePx": "0.010097",
                "markPx": "0.010089",
                "midPx": "0.010091",
                "impactPxs": [
                    "0.010089",
                    "0.010094"
                ],
                "dayBaseVlm": "208111122.0"
            },
            {
                "funding": "0.0000056062",
                "openInterest": "19352424.8000000007",
                "prevDayPx": "0.02689",
                "dayNtlVlm": "309408.7024149999",
                "premium": "0.0014119308",
                "oraclePx": "0.02833",
                "markPx": "0.02836",
                "midPx": "0.02838",
                "impactPxs": [
                    "0.02837",
                    "0.02841"
                ],
                "dayBaseVlm": "10948056.3999999985"
            },
            {
                "funding": "0.0000125",
                "openInterest": "195711060.1999999881",
                "prevDayPx": "0.31455",
                "dayNtlVlm": "40935172.4680689946",
                "premium": "-0.0002262078",
                "oraclePx": "0.30945",
                "markPx": "0.30939",
                "midPx": "0.30929",
                "impactPxs": [
                    "0.30929",
                    "0.30938"
                ],
                "dayBaseVlm": "128309615.9000000507"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.06206",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.06153",
                "markPx": "0.06157",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "17458788.0",
                "prevDayPx": "0.029664",
                "dayNtlVlm": "276382.419724",
                "premium": "-0.0002386635",
                "oraclePx": "0.02933",
                "markPx": "0.029305",
                "midPx": "0.029294",
                "impactPxs": [
                    "0.02926",
                    "0.029323"
                ],
                "dayBaseVlm": "9223573.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "109175682.0",
                "prevDayPx": "0.012692",
                "dayNtlVlm": "242226.7813099999",
                "premium": "0.0",
                "oraclePx": "0.012685",
                "markPx": "0.01269",
                "midPx": "0.012691",
                "impactPxs": [
                    "0.012675",
                    "0.012712"
                ],
                "dayBaseVlm": "18831134.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "14458656.0",
                "prevDayPx": "0.044031",
                "dayNtlVlm": "572863.0616359997",
                "premium": "-0.0006275494",
                "oraclePx": "0.047805",
                "markPx": "0.047731",
                "midPx": "0.047709",
                "impactPxs": [
                    "0.047658",
                    "0.047775"
                ],
                "dayBaseVlm": "12417526.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "108300270.0",
                "prevDayPx": "0.017561",
                "dayNtlVlm": "203705.967197",
                "premium": "0.0",
                "oraclePx": "0.01743",
                "markPx": "0.01743",
                "midPx": "0.017428",
                "impactPxs": [
                    "0.017417",
                    "0.017454"
                ],
                "dayBaseVlm": "11537249.0"
            },
            {
                "funding": "-0.0000411859",
                "openInterest": "8374873.7999999998",
                "prevDayPx": "0.49516",
                "dayNtlVlm": "1332738.8010890002",
                "premium": "-0.0003593624",
                "oraclePx": "0.47306",
                "markPx": "0.47243",
                "midPx": "0.47267",
                "impactPxs": [
                    "0.47243",
                    "0.47289"
                ],
                "dayBaseVlm": "2722774.0999999996"
            },
            {
                "funding": "0.0000100676",
                "openInterest": "22746806.0",
                "prevDayPx": "0.073646",
                "dayNtlVlm": "1006252.367599",
                "premium": "-0.0003228757",
                "oraclePx": "0.074332",
                "markPx": "0.074268",
                "midPx": "0.074273",
                "impactPxs": [
                    "0.074221",
                    "0.074308"
                ],
                "dayBaseVlm": "13377121.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "2333659.5999999996",
                "prevDayPx": "1.2643",
                "dayNtlVlm": "303425.72266",
                "premium": "0.0",
                "oraclePx": "1.2385",
                "markPx": "1.2377",
                "midPx": "1.23805",
                "impactPxs": [
                    "1.23752",
                    "1.23873"
                ],
                "dayBaseVlm": "238068.6"
            },
            {
                "funding": "-0.0000215446",
                "openInterest": "2029810.3999999999",
                "prevDayPx": "4.9521",
                "dayNtlVlm": "1274170.2166800003",
                "premium": "-0.0007888631",
                "oraclePx": "4.9565",
                "markPx": "4.9525",
                "midPx": "4.9519",
                "impactPxs": [
                    "4.95081",
                    "4.95259"
                ],
                "dayBaseVlm": "255495.3"
            },
            {
                "funding": "0.0000125",
                "openInterest": "11221864.1999999993",
                "prevDayPx": "0.16248",
                "dayNtlVlm": "501054.122368",
                "premium": "0.0",
                "oraclePx": "0.14516",
                "markPx": "0.14505",
                "midPx": "0.14502",
                "impactPxs": [
                    "0.14489",
                    "0.14516"
                ],
                "dayBaseVlm": "3255882.7999999993"
            },
            {
                "funding": "-0.0000767853",
                "openInterest": "90054532.0",
                "prevDayPx": "0.006853",
                "dayNtlVlm": "315207.473736",
                "premium": "-0.0013636364",
                "oraclePx": "0.0066",
                "markPx": "0.006589",
                "midPx": "0.006587",
                "impactPxs": [
                    "0.006584",
                    "0.006591"
                ],
                "dayBaseVlm": "46598437.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "23003598.0",
                "prevDayPx": "0.025032",
                "dayNtlVlm": "68076.271998",
                "premium": "-0.000536193",
                "oraclePx": "0.02611",
                "markPx": "0.02609",
                "midPx": "0.02607",
                "impactPxs": [
                    "0.026047",
                    "0.026096"
                ],
                "dayBaseVlm": "2627122.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1132895.04",
                "prevDayPx": "2.9355",
                "dayNtlVlm": "887861.0031830001",
                "premium": "-0.0008247571",
                "oraclePx": "2.7887",
                "markPx": "2.7865",
                "midPx": "2.7854",
                "impactPxs": [
                    "2.7839",
                    "2.7864"
                ],
                "dayBaseVlm": "304019.66"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.037555",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.03719",
                "markPx": "0.037555",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "-0.0008233345",
                "openInterest": "7894067.0",
                "prevDayPx": "0.87008",
                "dayNtlVlm": "18295236.5677020065",
                "premium": "-0.005257092",
                "oraclePx": "0.98914",
                "markPx": "0.98377",
                "midPx": "0.98309",
                "impactPxs": [
                    "0.98174",
                    "0.98394"
                ],
                "dayBaseVlm": "19779841.5000000037"
            },
            {
                "funding": "0.0000125",
                "openInterest": "19033892.0",
                "prevDayPx": "0.015267",
                "dayNtlVlm": "68283.341881",
                "premium": "0.0",
                "oraclePx": "0.01535",
                "markPx": "0.015345",
                "midPx": "0.015345",
                "impactPxs": [
                    "0.015326",
                    "0.015366"
                ],
                "dayBaseVlm": "4451907.0"
            },
            {
                "funding": "-0.0000789483",
                "openInterest": "3641358.0",
                "prevDayPx": "0.1428",
                "dayNtlVlm": "115934.15881",
                "premium": "-0.0004028269",
                "oraclePx": "0.1415",
                "markPx": "0.1412",
                "midPx": "0.14121",
                "impactPxs": [
                    "0.141095",
                    "0.141443"
                ],
                "dayBaseVlm": "804284.0"
            },
            {
                "funding": "-0.0001158583",
                "openInterest": "1681073.8",
                "prevDayPx": "2.5411",
                "dayNtlVlm": "2285337.4384300006",
                "premium": "-0.0005658852",
                "oraclePx": "2.3503",
                "markPx": "2.346",
                "midPx": "2.3473",
                "impactPxs": [
                    "2.34621",
                    "2.34897"
                ],
                "dayBaseVlm": "933554.8000000003"
            },
            {
                "funding": "0.0000125",
                "openInterest": "34580090.6000000015",
                "prevDayPx": "0.06732",
                "dayNtlVlm": "227838.782005",
                "premium": "0.0",
                "oraclePx": "0.06739",
                "markPx": "0.06736",
                "midPx": "0.06737",
                "impactPxs": [
                    "0.06733",
                    "0.0674"
                ],
                "dayBaseVlm": "3320999.6999999993"
            },
            {
                "funding": "-0.0000813721",
                "openInterest": "8108284.0",
                "prevDayPx": "0.48823",
                "dayNtlVlm": "980263.9665600006",
                "premium": "-0.0012659322",
                "oraclePx": "0.4629",
                "markPx": "0.46206",
                "midPx": "0.461915",
                "impactPxs": [
                    "0.461641",
                    "0.462314"
                ],
                "dayBaseVlm": "2051827.0"
            },
            {
                "funding": "-0.0002230195",
                "openInterest": "9222372.0",
                "prevDayPx": "0.066659",
                "dayNtlVlm": "709033.8169880001",
                "premium": "-0.0024751067",
                "oraclePx": "0.0703",
                "markPx": "0.07006",
                "midPx": "0.070065",
                "impactPxs": [
                    "0.069997",
                    "0.070126"
                ],
                "dayBaseVlm": "9931264.0"
            },
            {
                "funding": "-0.0000038894",
                "openInterest": "18090.118",
                "prevDayPx": "4678.4",
                "dayNtlVlm": "7210860.3193000006",
                "premium": "-0.0004221458",
                "oraclePx": "4737.7",
                "markPx": "4735.5",
                "midPx": "4735.65",
                "impactPxs": [
                    "4735.291",
                    "4735.7"
                ],
                "dayBaseVlm": "1536.063"
            },
            {
                "funding": "0.0000125",
                "openInterest": "46628834.0",
                "prevDayPx": "0.061775",
                "dayNtlVlm": "244288.86353",
                "premium": "0.0003894495",
                "oraclePx": "0.05649",
                "markPx": "0.056553",
                "midPx": "0.056548",
                "impactPxs": [
                    "0.056512",
                    "0.05661"
                ],
                "dayBaseVlm": "4072719.0"
            },
            {
                "funding": "-0.000066109",
                "openInterest": "19064602.0",
                "prevDayPx": "0.017474",
                "dayNtlVlm": "108760.664684",
                "premium": "-0.0017795637",
                "oraclePx": "0.01742",
                "markPx": "0.017366",
                "midPx": "0.017375",
                "impactPxs": [
                    "0.017358",
                    "0.017389"
                ],
                "dayBaseVlm": "6174965.0"
            },
            {
                "funding": "-0.0000542086",
                "openInterest": "6930542.0",
                "prevDayPx": "0.073522",
                "dayNtlVlm": "185485.6521470002",
                "premium": "-0.0012376743",
                "oraclePx": "0.073525",
                "markPx": "0.073385",
                "midPx": "0.073389",
                "impactPxs": [
                    "0.073353",
                    "0.073434"
                ],
                "dayBaseVlm": "2482458.0"
            },
            {
                "funding": "-0.0000314956",
                "openInterest": "2842578.0",
                "prevDayPx": "0.12157",
                "dayNtlVlm": "58882.10203",
                "premium": "-0.0004452055",
                "oraclePx": "0.1168",
                "markPx": "0.11658",
                "midPx": "0.11661",
                "impactPxs": [
                    "0.116506",
                    "0.116748"
                ],
                "dayBaseVlm": "492033.0"
            },
            {
                "funding": "-0.000100549",
                "openInterest": "57423680.0",
                "prevDayPx": "0.032629",
                "dayNtlVlm": "289022.8504020001",
                "premium": "-0.0010208817",
                "oraclePx": "0.032325",
                "markPx": "0.03226",
                "midPx": "0.032263",
                "impactPxs": [
                    "0.032236",
                    "0.032292"
                ],
                "dayBaseVlm": "8810068.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "16494042.0",
                "prevDayPx": "0.086337",
                "dayNtlVlm": "191686.6654320002",
                "premium": "0.0",
                "oraclePx": "0.084",
                "markPx": "0.083848",
                "midPx": "0.083888",
                "impactPxs": [
                    "0.083751",
                    "0.084067"
                ],
                "dayBaseVlm": "2231814.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "85166532.0",
                "prevDayPx": "0.004888",
                "dayNtlVlm": "260382.63934",
                "premium": "-0.0013747054",
                "oraclePx": "0.005092",
                "markPx": "0.005079",
                "midPx": "0.005079",
                "impactPxs": [
                    "0.005076",
                    "0.005085"
                ],
                "dayBaseVlm": "50538539.0"
            },
            {
                "funding": "0.0",
                "openInterest": "0.0",
                "prevDayPx": "0.060054",
                "dayNtlVlm": "0.0",
                "premium": null,
                "oraclePx": "0.06012",
                "markPx": "0.060367",
                "midPx": null,
                "impactPxs": null,
                "dayBaseVlm": "0.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1098216.0",
                "prevDayPx": "0.35089",
                "dayNtlVlm": "75021.77852",
                "premium": "0.0",
                "oraclePx": "0.3326",
                "markPx": "0.33205",
                "midPx": "0.33227",
                "impactPxs": [
                    "0.332003",
                    "0.332679"
                ],
                "dayBaseVlm": "220962.0"
            },
            {
                "funding": "-0.0000034429",
                "openInterest": "21565360.0",
                "prevDayPx": "0.012698",
                "dayNtlVlm": "150945.758255",
                "premium": "-0.0002435065",
                "oraclePx": "0.01232",
                "markPx": "0.0123",
                "midPx": "0.012308",
                "impactPxs": [
                    "0.012297",
                    "0.012317"
                ],
                "dayBaseVlm": "11899391.0"
            },
            {
                "funding": "-0.0002731601",
                "openInterest": "52628128.0",
                "prevDayPx": "0.09006",
                "dayNtlVlm": "16922735.9779790044",
                "premium": "-0.0012026144",
                "oraclePx": "0.095625",
                "markPx": "0.095355",
                "midPx": "0.095387",
                "impactPxs": [
                    "0.095352",
                    "0.09551"
                ],
                "dayBaseVlm": "174156298.0"
            },
            {
                "funding": "-0.0000278873",
                "openInterest": "5272824.0",
                "prevDayPx": "0.35573",
                "dayNtlVlm": "405638.31689",
                "premium": "-0.000376898",
                "oraclePx": "0.3688",
                "markPx": "0.36845",
                "midPx": "0.368565",
                "impactPxs": [
                    "0.368447",
                    "0.368661"
                ],
                "dayBaseVlm": "1125727.0"
            },
            {
                "funding": "0.000004628",
                "openInterest": "19344323566.0",
                "prevDayPx": "0.002491",
                "dayNtlVlm": "31069698.0730839968",
                "premium": "-0.0003888025",
                "oraclePx": "0.002572",
                "markPx": "0.002569",
                "midPx": "0.002569",
                "impactPxs": [
                    "0.002569",
                    "0.002571"
                ],
                "dayBaseVlm": "12172995235.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1879896.0",
                "prevDayPx": "0.37617",
                "dayNtlVlm": "113196.3245",
                "premium": "-0.0001257532",
                "oraclePx": "0.3817",
                "markPx": "0.38133",
                "midPx": "0.38135",
                "impactPxs": [
                    "0.381151",
                    "0.381652"
                ],
                "dayBaseVlm": "293711.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1662114.0",
                "prevDayPx": "0.33779",
                "dayNtlVlm": "25165.59035",
                "premium": "0.0",
                "oraclePx": "0.34825",
                "markPx": "0.34844",
                "midPx": "0.348565",
                "impactPxs": [
                    "0.347242",
                    "0.34996"
                ],
                "dayBaseVlm": "73069.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "289653588.0",
                "prevDayPx": "0.12815",
                "dayNtlVlm": "8237710.7814300032",
                "premium": "-0.0001652893",
                "oraclePx": "0.12705",
                "markPx": "0.12697",
                "midPx": "0.126995",
                "impactPxs": [
                    "0.126933",
                    "0.127029"
                ],
                "dayBaseVlm": "62460070.0"
            },
            {
                "funding": "-0.0000916318",
                "openInterest": "150963626.0",
                "prevDayPx": "0.16061",
                "dayNtlVlm": "4299574.694050001",
                "premium": "-0.0008451573",
                "oraclePx": "0.1621",
                "markPx": "0.16194",
                "midPx": "0.161925",
                "impactPxs": [
                    "0.161879",
                    "0.161963"
                ],
                "dayBaseVlm": "26271401.0"
            },
            {
                "funding": "-0.0000321236",
                "openInterest": "218349202.0",
                "prevDayPx": "0.00568",
                "dayNtlVlm": "228544.301786",
                "premium": "-0.0010425717",
                "oraclePx": "0.005755",
                "markPx": "0.005747",
                "midPx": "0.005748",
                "impactPxs": [
                    "0.005744",
                    "0.005749"
                ],
                "dayBaseVlm": "40233023.0"
            },
            {
                "funding": "-0.0001078722",
                "openInterest": "18966342.0",
                "prevDayPx": "0.063662",
                "dayNtlVlm": "441782.5365190001",
                "premium": "-0.0014229249",
                "oraclePx": "0.06325",
                "markPx": "0.063099",
                "midPx": "0.063121",
                "impactPxs": [
                    "0.063088",
                    "0.06316"
                ],
                "dayBaseVlm": "6777604.0"
            },
            {
                "funding": "-0.0000254831",
                "openInterest": "49073220.0",
                "prevDayPx": "0.63045",
                "dayNtlVlm": "9383764.8491300009",
                "premium": "0.0",
                "oraclePx": "0.6091",
                "markPx": "0.60884",
                "midPx": "0.60901",
                "impactPxs": [
                    "0.608793",
                    "0.609165"
                ],
                "dayBaseVlm": "15116352.0"
            },
            {
                "funding": "-0.0003723848",
                "openInterest": "10299980.0",
                "prevDayPx": "0.26924",
                "dayNtlVlm": "2105429.3234799993",
                "premium": "-0.0032583561",
                "oraclePx": "0.28542",
                "markPx": "0.28421",
                "midPx": "0.28428",
                "impactPxs": [
                    "0.284109",
                    "0.28449"
                ],
                "dayBaseVlm": "7320170.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "79468980.0",
                "prevDayPx": "0.047113",
                "dayNtlVlm": "627662.8903060001",
                "premium": "-0.000021561",
                "oraclePx": "0.04638",
                "markPx": "0.046347",
                "midPx": "0.046294",
                "impactPxs": [
                    "0.046231",
                    "0.046379"
                ],
                "dayBaseVlm": "13059028.0"
            },
            {
                "funding": "-0.000094695",
                "openInterest": "1077750.0",
                "prevDayPx": "0.78261",
                "dayNtlVlm": "246471.27713",
                "premium": "-0.00097668",
                "oraclePx": "0.7976",
                "markPx": "0.79629",
                "midPx": "0.796475",
                "impactPxs": [
                    "0.796168",
                    "0.796821"
                ],
                "dayBaseVlm": "309724.0"
            },
            {
                "funding": "0.000036413",
                "openInterest": "112947966.0",
                "prevDayPx": "0.014568",
                "dayNtlVlm": "310798.6974950001",
                "premium": "0.0",
                "oraclePx": "0.0143",
                "markPx": "0.0143",
                "midPx": "0.01431",
                "impactPxs": [
                    "0.014291",
                    "0.014321"
                ],
                "dayBaseVlm": "21307681.0"
            },
            {
                "funding": "0.0000112439",
                "openInterest": "2585406.0",
                "prevDayPx": "0.37826",
                "dayNtlVlm": "429648.45213",
                "premium": "0.0",
                "oraclePx": "0.3734",
                "markPx": "0.37291",
                "midPx": "0.372925",
                "impactPxs": [
                    "0.372412",
                    "0.373531"
                ],
                "dayBaseVlm": "1140438.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "13181444.0",
                "prevDayPx": "0.12087",
                "dayNtlVlm": "143727.55211",
                "premium": "-0.0003216495",
                "oraclePx": "0.12125",
                "markPx": "0.12107",
                "midPx": "0.1211",
                "impactPxs": [
                    "0.120999",
                    "0.121211"
                ],
                "dayBaseVlm": "1189676.0"
            },
            {
                "funding": "-0.0000106353",
                "openInterest": "337961.08",
                "prevDayPx": "370.41",
                "dayNtlVlm": "41276123.5452000052",
                "premium": "-0.0006204653",
                "oraclePx": "359.73",
                "markPx": "359.48",
                "midPx": "359.47",
                "impactPxs": [
                    "359.404",
                    "359.5068"
                ],
                "dayBaseVlm": "111657.93"
            },
            {
                "funding": "0.0000125",
                "openInterest": "1721100546.0",
                "prevDayPx": "0.020748",
                "dayNtlVlm": "6493538.6054920023",
                "premium": "0.0",
                "oraclePx": "0.019725",
                "markPx": "0.019725",
                "midPx": "0.01973",
                "impactPxs": [
                    "0.01972",
                    "0.019742"
                ],
                "dayBaseVlm": "321236410.0"
            },
            {
                "funding": "-0.0000069481",
                "openInterest": "16697398.0",
                "prevDayPx": "0.29026",
                "dayNtlVlm": "2344144.9273700002",
                "premium": "0.0",
                "oraclePx": "0.27355",
                "markPx": "0.27321",
                "midPx": "0.27334",
                "impactPxs": [
                    "0.273177",
                    "0.273559"
                ],
                "dayBaseVlm": "8237425.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "19218570.0",
                "prevDayPx": "0.20901",
                "dayNtlVlm": "368025.4517499999",
                "premium": "0.0",
                "oraclePx": "0.20851",
                "markPx": "0.20925",
                "midPx": "0.2093",
                "impactPxs": [
                    "0.202492",
                    "0.21365"
                ],
                "dayBaseVlm": "1759487.0"
            },
            {
                "funding": "-0.0000104586",
                "openInterest": "29432830.0",
                "prevDayPx": "0.11248",
                "dayNtlVlm": "1651408.521810001",
                "premium": "0.0",
                "oraclePx": "0.12751",
                "markPx": "0.12738",
                "midPx": "0.127335",
                "impactPxs": [
                    "0.12717",
                    "0.127551"
                ],
                "dayBaseVlm": "14221400.0"
            },
            {
                "funding": "0.0000047274",
                "openInterest": "741014.6",
                "prevDayPx": "4.1074",
                "dayNtlVlm": "4920731.8345599975",
                "premium": "0.0",
                "oraclePx": "3.713",
                "markPx": "3.7101",
                "midPx": "3.7107",
                "impactPxs": [
                    "3.71002",
                    "3.71347"
                ],
                "dayBaseVlm": "1212353.1999999997"
            },
            {
                "funding": "-0.0000195852",
                "openInterest": "4707582.0",
                "prevDayPx": "0.49568",
                "dayNtlVlm": "760122.4081499999",
                "premium": "-0.0006316612",
                "oraclePx": "0.48602",
                "markPx": "0.48536",
                "midPx": "0.485335",
                "impactPxs": [
                    "0.485042",
                    "0.485713"
                ],
                "dayBaseVlm": "1532756.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "105196164.0",
                "prevDayPx": "0.016164",
                "dayNtlVlm": "248442.791641",
                "premium": "0.0",
                "oraclePx": "0.016382",
                "markPx": "0.016375",
                "midPx": "0.016375",
                "impactPxs": [
                    "0.016361",
                    "0.016385"
                ],
                "dayBaseVlm": "14841355.0"
            },
            {
                "funding": "-0.0000187736",
                "openInterest": "39915018.0",
                "prevDayPx": "0.027602",
                "dayNtlVlm": "1679969.8785609996",
                "premium": "-0.0007497858",
                "oraclePx": "0.028008",
                "markPx": "0.027965",
                "midPx": "0.027967",
                "impactPxs": [
                    "0.027952",
                    "0.027987"
                ],
                "dayBaseVlm": "56654908.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "26971060.0",
                "prevDayPx": "1.7342",
                "dayNtlVlm": "16161286.4281000011",
                "premium": "0.0003717184",
                "oraclePx": "1.676",
                "markPx": "1.677",
                "midPx": "1.67715",
                "impactPxs": [
                    "1.676623",
                    "1.678041"
                ],
                "dayBaseVlm": "9585818.0"
            },
            {
                "funding": "0.0000125",
                "openInterest": "59604.782",
                "prevDayPx": "636.09",
                "dayNtlVlm": "36777909.3272300065",
                "premium": "-0.0006641739",
                "oraclePx": "585.69",
                "markPx": "585.63",
                "midPx": "585.12",
                "impactPxs": [
                    "584.815",
                    "585.301"
                ],
                "dayBaseVlm": "58948.927"
            },
            {
                "funding": "-0.0014399281",
                "openInterest": "1954920.0000000002",
                "prevDayPx": "1.7487",
                "dayNtlVlm": "10377028.9306500014",
                "premium": "-0.0101427868",
                "oraclePx": "2.031",
                "markPx": "2.0064",
                "midPx": "2.00905",
                "impactPxs": [
                    "2.00819",
                    "2.0104"
                ],
                "dayBaseVlm": "5858506.0"
            },
            {
                "funding": "-0.00017487",
                "openInterest": "186302.3",
                "prevDayPx": "86.469",
                "dayNtlVlm": "12614237.4955299962",
                "premium": "-0.0023807543",
                "oraclePx": "72.12",
                "markPx": "71.909",
                "midPx": "71.922",
                "impactPxs": [
                    "71.8689",
                    "71.9483"
                ],
                "dayBaseVlm": "161630.4"
            }
        ]
]"#;
