//! Telemetry setup for OpenTelemetry integration
//! Phase 4: Production observability

use anyhow::Result;

/// Initialize OpenTelemetry if enabled
///
/// # Environment Variables
///
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP endpoint (e.g., http://localhost:4317)
/// - `OTEL_SERVICE_NAME`: Service name (default: semantica-task-engine)
///
/// # Example
///
/// ```text
/// OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
/// OTEL_SERVICE_NAME=semantica-dev \
///     ./semantica
/// ```
pub fn init_telemetry() -> Result<()> {
    // Check if OpenTelemetry is configured
    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_err() {
        tracing::debug!("OpenTelemetry not configured (OTEL_EXPORTER_OTLP_ENDPOINT not set)");
        return Ok(());
    }

    #[cfg(feature = "telemetry")]
    {
        init_telemetry_impl()?;
    }

    #[cfg(not(feature = "telemetry"))]
    {
        tracing::warn!("OpenTelemetry endpoint set but feature 'telemetry' not enabled");
        tracing::warn!("Rebuild with: cargo build --features telemetry");
    }

    Ok(())
}

#[cfg(feature = "telemetry")]
fn init_telemetry_impl() -> Result<()> {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::Tracer;

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "semantica-task-engine".to_string());

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")?;

    tracing::info!(
        service_name = %service_name,
        endpoint = %endpoint,
        "Initializing OpenTelemetry"
    );

    let tracer: Tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&endpoint),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?
        .tracer(service_name);

    // Register with tracing-subscriber
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    use tracing_subscriber::layer::SubscriberExt;
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(telemetry_layer))?;

    tracing::info!("OpenTelemetry initialized successfully");

    Ok(())
}
