import http from 'k6/http';

export const options = {
  vus: 5000,
  duration: '120s',
  summaryTrendStats: ['min', 'med', 'avg', 'p(90)', 'p(95)', 'p(99)', 'max'],
};

export default function () {
  http.get('http://localhost:8080/api/v1/ping');
}
