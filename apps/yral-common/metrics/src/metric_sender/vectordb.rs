// vectordb metric sender removed — fly.io vector DB is no longer available.
// This module was dead code (VectorDbMetricTx was never used by any app).

#[cfg(not(feature = "js"))]
impl super::MetricEventTx for VectorDbMetricTx {
    type Error = reqwest::Error;

    async fn push<M: Metric + Send>(&self, ev: MetricEvent<M>) -> Result<(), Self::Error> {
        self.push_inner(ev).await
    }

    async fn push_list<M: Metric + Send>(&self, ev: MetricEventList<M>) -> Result<(), Self::Error> {
        self.push_list_inner(ev).await
    }
}
