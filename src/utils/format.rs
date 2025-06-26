use crate::finance::models::Ticker;
use arrow::{
    array::{ArrayRef, Int64Array, RecordBatch, StringArray},
    datatypes::{DataType, Field, Schema, SchemaRef},
};
use std::sync::Arc;

pub fn ticker_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("symbol", DataType::Utf8, false),
        Field::new("exchange", DataType::Utf8, false),
        Field::new("description", DataType::Utf8, true),
        Field::new("currency", DataType::Utf8, true),
        Field::new("country", DataType::Utf8, true),
        Field::new("market_type", DataType::Utf8, true),
        Field::new("industry", DataType::Utf8, true),
        Field::new("sector", DataType::Utf8, true),
        Field::new("founded", DataType::Int64, true),
    ]))
}

/// Convert Vec<Ticker> to Arrow RecordBatch
pub fn to_batch(tickers: Vec<Ticker>) -> arrow::error::Result<RecordBatch> {
    let schema = ticker_schema();

    let symbols: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.symbol.as_str())
            .collect::<Vec<_>>(),
    ));

    let exchanges: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.exchange.as_str())
            .collect::<Vec<_>>(),
    ));

    let descriptions: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.description.as_deref())
            .collect::<Vec<_>>(),
    ));

    let currencies: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.currency.as_deref())
            .collect::<Vec<_>>(),
    ));

    let countries: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.country.as_deref())
            .collect::<Vec<_>>(),
    ));

    let market_types: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.market_type.as_deref())
            .collect::<Vec<_>>(),
    ));

    let industries: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.industry.as_deref())
            .collect::<Vec<_>>(),
    ));

    let sectors: ArrayRef = Arc::new(StringArray::from(
        tickers
            .iter()
            .map(|t| t.sector.as_deref())
            .collect::<Vec<_>>(),
    ));

    let founded: ArrayRef = Arc::new(Int64Array::from(
        tickers.iter().map(|t| t.founded).collect::<Vec<_>>(),
    ));

    RecordBatch::try_new(
        schema,
        vec![
            symbols,
            exchanges,
            descriptions,
            currencies,
            countries,
            market_types,
            industries,
            sectors,
            founded,
        ],
    )
}

/// Export tickers to Parquet file
pub fn save_parquet(tickers: Vec<Ticker>, path: &str) -> anyhow::Result<()> {
    use parquet::arrow::ArrowWriter;
    use std::fs::File;

    let batch = to_batch(tickers)?;
    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None)?;

    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

/// Export tickers in batches to Parquet (for large datasets)
pub fn save_parquet_batched(
    tickers: Vec<Ticker>,
    path: &str,
    batch_size: usize,
) -> anyhow::Result<()> {
    use parquet::arrow::ArrowWriter;
    use std::fs::File;

    if tickers.is_empty() {
        return Ok(());
    }

    let schema = ticker_schema();
    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, schema, None)?;

    for chunk in tickers.chunks(batch_size) {
        let batch = to_batch(chunk.to_vec())?;
        writer.write(&batch)?;
    }

    writer.close()?;
    Ok(())
}

pub fn from_batch(batch: &RecordBatch) -> anyhow::Result<Vec<Ticker>> {
    use arrow::array::*;

    let symbols = batch
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let exchanges = batch
        .column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let descriptions = batch
        .column(2)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let currencies = batch
        .column(3)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let countries = batch
        .column(4)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let market_types = batch
        .column(5)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let industries = batch
        .column(6)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let sectors = batch
        .column(7)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let founded = batch
        .column(8)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();

    let mut tickers = Vec::with_capacity(batch.num_rows());

    for i in 0..batch.num_rows() {
        tickers.push(Ticker {
            symbol: symbols.value(i).to_string(),
            exchange: exchanges.value(i).to_string(),
            description: if descriptions.is_null(i) {
                None
            } else {
                Some(descriptions.value(i).to_string())
            },
            currency: if currencies.is_null(i) {
                None
            } else {
                Some(currencies.value(i).to_string())
            },
            country: if countries.is_null(i) {
                None
            } else {
                Some(countries.value(i).to_string())
            },
            market_type: if market_types.is_null(i) {
                None
            } else {
                Some(market_types.value(i).to_string())
            },
            industry: if industries.is_null(i) {
                None
            } else {
                Some(industries.value(i).to_string())
            },
            sector: if sectors.is_null(i) {
                None
            } else {
                Some(sectors.value(i).to_string())
            },
            founded: if founded.is_null(i) {
                None
            } else {
                Some(founded.value(i))
            },
        });
    }

    Ok(tickers)
}
