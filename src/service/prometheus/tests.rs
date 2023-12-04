use crate::service::prometheus::types::PrometheusResponse;
use serde_json::from_str;

#[test]
fn test_decode_prometheus_response() {
    // Mock data, representative of what the Prometheus API might return
    let mock_response = r#"
        {
            "data": {
                "resultType": "matrix",
                "result": [
                    {
                        "metric": {"__name__": "up", "job": "api"},
                        "values": [
                            [1635171094.561, "1"],
                            [1635171394.561, "0"]
                        ]
                    }
                ]
            }
        }
        "#;

    // Attempt to decode mock prometheus http response into `PrometheusResponse`
    let result: Result<PrometheusResponse, _> = from_str(mock_response);

    // Check the result
    match result {
        Ok(_) => {
            // For now, we're just checking that the decoding succeeded
            // But we could add assertions for specific fields if needed
            //
            // assert_eq!(response.data, PrometheusData::Matrix(Vec::new())); // You'd compare with the actual data you mocked
        }
        Err(err) => {
            panic!("Decoding failed: {}", err);
        }
    }
}
