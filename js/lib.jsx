import 'bootstrap';
import React, { useState, useEffect } from 'react';
import { render } from 'react-dom';
import PropTypes from 'prop-types';

const CategorySelector = ({ selectedCategory, categories, onChange }) => {
  const change = (e) => {
    onChange(e.target.value);
  };

  return (
    <form>
      <div className="mb-3">
        <label htmlFor="categorySelection" className="form-label">Leeftijdscategorie</label>
        <select className="form-select" id="categorySelection" defaultValue={selectedCategory} onChange={change}>
          <option label="Kies een categorie" />
          {categories.map((category) => <option key={category}>{category}</option>)}
        </select>
      </div>
    </form>
  );
};

CategorySelector.propTypes = {
  categories: PropTypes.arrayOf(PropTypes.string).isRequired,
  selectedCategory: PropTypes.string.isRequired,
  onChange: PropTypes.func.isRequired,
};

const RankingResult = ({ score, place }) => (
  <div className="col">
    {score}
    <br />
    <span className="text-muted">
      (
      {place}
      )
    </span>
  </div>
);

RankingResult.propTypes = {
  score: PropTypes.number,
  place: PropTypes.oneOfType([PropTypes.number, PropTypes.string]),
};

RankingResult.defaultProps = {
  score: 0,
  place: '-',
};

const RankingEntry = ({ entry }) => (
  <div className="row mt-lg-2 mt-3 pb-2">
    <div className="col-lg-5">
      <div className="row gx-3">
        <div className="col-2 text-end">
          {entry.place}
        </div>
        <div className="col-10">
          {entry.name}
          {' '}
          <br />
          {' '}
          <span className="text-muted">{entry.club}</span>
        </div>
      </div>
    </div>
    <div className="col-lg mt-1 mt-lg-0 text-end">
      <div className="row">
        {entry.scores.map(
          (result) => (
            <RankingResult
              key={result.eventId}
              score={result.score || undefined}
              place={result.place || undefined}
            />
          ),
        )}
        <div className="col">
          <strong>{entry.totalScore}</strong>
        </div>
      </div>
    </div>
  </div>
);

RankingEntry.propTypes = {
  entry: PropTypes.shape({
    place: PropTypes.string.isRequired,
    name: PropTypes.string.isRequired,
    club: PropTypes.string.isRequired,
    totalScore: PropTypes.number.isRequired,
    scores: PropTypes.arrayOf(PropTypes.shape({
      eventId: PropTypes.number.isRequired,
      score: PropTypes.number,
      place: PropTypes.number,
    })),
  }).isRequired,
};

const Ranking = ({ categories, cup, season }) => {
  const [selectedCategory, setSelectedCategory] = useState('');
  const [isLoading, setLoading] = useState(false);
  const [ranking, setRanking] = useState([]);

  const generateRanking = (data) => {
    const entries = [];
    let place = 0;
    let previousScore = 5001;
    data.forEach((entry) => {
      place += 1;
      let thisPlace;
      if (entry.totalScore < previousScore) {
        thisPlace = `${place}.`;
      } else {
        thisPlace = '-';
      }
      previousScore = entry.totalScore;
      entries.push({ ...entry, place: thisPlace });
    });
    setRanking(entries);
  };

  useEffect(() => {
    if (!selectedCategory) {
      setRanking([]);
      return;
    }
    setLoading(true);
    const url = new URL('./cgi-bin/cup-cgi', window.location);
    url.searchParams.set('cup', cup);
    url.searchParams.set('season', season);
    url.searchParams.set('ageClass', selectedCategory);

    fetch(url)
      .then((response) => response.json())
      .then((data) => { setLoading(false); return data; })
      .then(generateRanking);
  }, [selectedCategory]);

  return (
    <>
      <CategorySelector
        categories={categories}
        selectedCategory={selectedCategory}
        onChange={setSelectedCategory}
      />
      {ranking.map((entry) => <RankingEntry key={`${entry.name}${entry.club}`} entry={entry} />)}
      {isLoading && (
        <div className="text-center">
          <div className="spinner-border" role="status">
            <span className="visually-hidden">Loading...</span>
          </div>
        </div>
      )}
      {selectedCategory
        && !isLoading
        && ranking.length === 0
        && (
          <div className="alert alert-info">
            Geen resultaten voor
            {` ${selectedCategory}`}
            .
          </div>
        )}
    </>
  );
};

Ranking.propTypes = {
  categories: PropTypes.arrayOf(PropTypes.string).isRequired,
  cup: PropTypes.string.isRequired,
  season: PropTypes.string.isRequired,
};

const categories = [];
document.querySelectorAll('#ranking span').forEach((category) => {
  categories.push(category.textContent);
});

const rankingContainer = document.getElementById('ranking');
if (rankingContainer) {
  render(
    <Ranking
      categories={categories}
      cup={rankingContainer.dataset.cup}
      season={rankingContainer.dataset.season}
    />,
    rankingContainer,
  );
}
