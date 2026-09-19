import http from 'k6/http';
import { check } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 100 },
    { duration: '30s', target: 500 },
    { duration: '30s', target: 1000 },
    { duration: '30s', target: 2000 },
    { duration: '30s', target: 5000 },
    { duration: '60s', target: 10000 },
    { duration: '30s', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(99)<50'],
    http_req_failed: ['rate<0.01'],
  },
};

export default function () {
  const res = http.get('http://localhost:8080/api/v1/ping');
  check(res, {
    'status OK': (r) => r.status === 200,
    'latency < 10ms': (r) => r.timings.duration < 10,
  });
}
