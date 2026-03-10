As a data analyst that works with timeseries data, like stocks, weather, or sensor measurements, I want to be able to 
plot multiple time series on the same plot using separate lines to represent them.

## Example

- Show the daily stock price of AAPL and GOOG for the past year (x=day, y=price, group=ticker).
- Show the hourly temperature for every day of the past year (x=hour, y=temperature, group=day).

## Implementation Notes

This requires adding a GEOM LINE that supports the `x` and `y` aesthetics, and also a `group` aesthetic to control which observations are connected in each line.
